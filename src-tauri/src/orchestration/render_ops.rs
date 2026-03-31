use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use reeln_config::{AppConfig, RenderProfile};
use reeln_media::{ConcatOptions, MediaBackend, RenderPlan, RenderResult};
use reeln_state::RenderEntry;

use super::progress::ProgressReporter;

/// Optional overrides for render profile parameters.
#[derive(Debug, Clone, Default)]
pub struct RenderOverrides {
    pub crop_mode: Option<String>,
    pub scale: Option<f64>,
    pub speed: Option<f64>,
    pub smart: Option<bool>,
    pub anchor_x: Option<f64>,
    pub anchor_y: Option<f64>,
    pub pad_color: Option<String>,
    pub zoom_frames: Option<u32>,
}

/// A single item in a render iteration queue.
#[derive(Debug, Clone)]
pub struct IterationItem {
    pub profile_name: String,
    pub overrides: Option<RenderOverrides>,
}

/// Apply overrides onto a cloned profile.
fn apply_overrides(profile: &mut RenderProfile, overrides: &RenderOverrides) {
    if let Some(ref cm) = overrides.crop_mode {
        profile.crop_mode = Some(cm.clone());
    }
    if let Some(s) = overrides.scale {
        profile.scale = Some(s);
    }
    if let Some(sp) = overrides.speed {
        profile.speed = Some(sp);
    }
    if let Some(sm) = overrides.smart {
        profile.smart = Some(sm);
    }
    if let Some(ax) = overrides.anchor_x {
        profile.anchor_x = Some(ax);
    }
    if let Some(ay) = overrides.anchor_y {
        profile.anchor_y = Some(ay);
    }
    if let Some(ref pc) = overrides.pad_color {
        profile.pad_color = Some(pc.clone());
    }
}

/// Build a full-frame `RenderPlan` — no crop/scale, just speed + LUT + encoding.
fn build_apply_plan(
    input: &Path,
    output: &Path,
    profile: &RenderProfile,
    config: &AppConfig,
) -> RenderPlan {
    let video_codec = profile
        .codec
        .clone()
        .unwrap_or_else(|| config.video.codec.clone());
    let crf = profile.crf.unwrap_or(config.video.crf);
    let preset = profile.preset.clone().or(Some(config.video.preset.clone()));
    let audio_codec = profile
        .audio_codec
        .clone()
        .unwrap_or_else(|| config.video.audio_codec.clone());
    let audio_bitrate = profile
        .audio_bitrate
        .as_ref()
        .and_then(|s| s.trim_end_matches('k').parse::<u32>().ok());

    let mut filters = Vec::new();
    let mut audio_filter = None;

    // Speed filter (no crop/scale): prefer speed_segments over scalar speed
    if let Some(ref segments) = profile.speed_segments {
        if !segments.is_empty() {
            let mut vparts = Vec::new();
            let mut aparts = Vec::new();
            let mut start = 0.0_f64;
            for (i, seg) in segments.iter().enumerate() {
                let end_clause = if let Some(until) = seg.until {
                    format!(":end={until}")
                } else {
                    String::new()
                };
                let pts_factor = 1.0 / seg.speed;
                vparts.push(format!(
                    "[0:v]trim=start={start}{end_clause},setpts={pts_factor:.4}*(PTS-STARTPTS)[v{i}]"
                ));
                aparts.push(format!(
                    "[0:a]atrim=start={start}{end_clause},asetpts=PTS-STARTPTS,atempo={speed}[a{i}]",
                    speed = seg.speed,
                ));
                if let Some(until) = seg.until {
                    start = until;
                }
            }
            let n = segments.len();
            let v_inputs: String = (0..n).map(|i| format!("[v{i}]")).collect();
            let a_inputs: String = (0..n).map(|i| format!("[a{i}]")).collect();
            let mut fc_parts: Vec<String> = Vec::new();
            fc_parts.extend(vparts);
            fc_parts.extend(aparts);
            fc_parts.push(format!(
                "{v_inputs}{a_inputs}concat=n={n}:v=1:a=1[vfinal][aout]"
            ));
            return RenderPlan {
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                video_codec,
                crf,
                preset,
                audio_codec,
                audio_bitrate,
                filters,
                filter_complex: Some(fc_parts.join(";")),
                audio_filter: None,
            };
        }
    }
    if let Some(speed) = profile.speed {
        if (speed - 1.0).abs() > f64::EPSILON {
            let pts_factor = 1.0 / speed;
            filters.push(format!("setpts={pts_factor:.4}*PTS"));
            audio_filter = Some(format!("atempo={speed}"));
        }
    }

    // LUT filter
    if let Some(ref lut) = profile.lut {
        filters.push(format!("lut3d={lut}"));
    }

    RenderPlan {
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        video_codec,
        crf,
        preset,
        audio_codec,
        audio_bitrate,
        filters,
        filter_complex: None,
        audio_filter,
    }
}

/// Build a `RenderPlan` from a `RenderProfile` and config defaults.
fn build_render_plan(
    input: &Path,
    output: &Path,
    profile: &RenderProfile,
    config: &AppConfig,
) -> RenderPlan {
    let video_codec = profile
        .codec
        .clone()
        .unwrap_or_else(|| config.video.codec.clone());
    let crf = profile.crf.unwrap_or(config.video.crf);
    let preset = profile.preset.clone().or(Some(config.video.preset.clone()));
    let audio_codec = profile
        .audio_codec
        .clone()
        .unwrap_or_else(|| config.video.audio_codec.clone());
    let audio_bitrate = profile
        .audio_bitrate
        .as_ref()
        .and_then(|s| s.trim_end_matches('k').parse::<u32>().ok());

    let mut filters = Vec::new();

    // Scale/crop filter
    if let (Some(w), Some(h)) = (profile.width, profile.height) {
        match profile.crop_mode.as_deref() {
            Some("crop") => {
                filters.push(format!("crop={w}:{h}"));
            }
            Some("pad") => {
                let pad_color = profile.pad_color.as_deref().unwrap_or("black");
                filters.push(format!(
                    "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color={pad_color}"
                ));
            }
            _ => {
                filters.push(format!("scale={w}:{h}"));
            }
        }
    } else if let Some(w) = profile.width {
        filters.push(format!("scale={w}:-2"));
    } else if let Some(h) = profile.height {
        filters.push(format!("scale=-2:{h}"));
    }

    // Speed filter: prefer speed_segments over scalar speed
    let mut audio_filter = None;
    if let Some(ref segments) = profile.speed_segments {
        if !segments.is_empty() {
            // Build per-segment trim+setpts chains and concatenate them
            let mut vparts = Vec::new();
            let mut aparts = Vec::new();
            let mut start = 0.0_f64;
            for (i, seg) in segments.iter().enumerate() {
                let end_clause = if let Some(until) = seg.until {
                    format!(":end={until}")
                } else {
                    String::new()
                };
                let pts_factor = 1.0 / seg.speed;
                vparts.push(format!(
                    "[0:v]trim=start={start}{end_clause},setpts={pts_factor:.4}*(PTS-STARTPTS)[v{i}]"
                ));
                aparts.push(format!(
                    "[0:a]atrim=start={start}{end_clause},asetpts=PTS-STARTPTS,atempo={speed}[a{i}]",
                    speed = seg.speed,
                ));
                if let Some(until) = seg.until {
                    start = until;
                }
            }
            let n = segments.len();
            let v_inputs: String = (0..n).map(|i| format!("[v{i}]")).collect();
            let a_inputs: String = (0..n).map(|i| format!("[a{i}]")).collect();
            let mut fc_parts: Vec<String> = Vec::new();
            fc_parts.extend(vparts);
            fc_parts.extend(aparts);
            // Prepend any scale/crop filters to each video segment
            if !filters.is_empty() {
                let filter_str = filters.join(",");
                // Apply scale/crop to the concat output
                fc_parts.push(format!(
                    "{v_inputs}{a_inputs}concat=n={n}:v=1:a=1[vout][aout];[vout]{filter_str}[vfinal]"
                ));
                // Clear filters since they're in the filter_complex now
                filters.clear();
                return RenderPlan {
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    video_codec,
                    crf,
                    preset,
                    audio_codec,
                    audio_bitrate,
                    filters,
                    filter_complex: Some(fc_parts.join(";")),
                    audio_filter: None,
                };
            } else {
                fc_parts.push(format!(
                    "{v_inputs}{a_inputs}concat=n={n}:v=1:a=1[vfinal][aout]"
                ));
                return RenderPlan {
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    video_codec,
                    crf,
                    preset,
                    audio_codec,
                    audio_bitrate,
                    filters,
                    filter_complex: Some(fc_parts.join(";")),
                    audio_filter: None,
                };
            }
        }
    }
    if let Some(speed) = profile.speed {
        if (speed - 1.0).abs() > f64::EPSILON {
            let pts_factor = 1.0 / speed;
            filters.push(format!("setpts={pts_factor:.4}*PTS"));
            audio_filter = Some(format!("atempo={speed}"));
        }
    }

    // LUT filter
    if let Some(ref lut) = profile.lut {
        filters.push(format!("lut3d={lut}"));
    }

    RenderPlan {
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        video_codec,
        crf,
        preset,
        audio_codec,
        audio_bitrate,
        filters,
        filter_complex: None,
        audio_filter,
    }
}

/// Render a short from a clip using a named render profile, with optional overrides.
///
/// `mode` controls the render strategy:
/// - `"short"` (default): applies crop/scale from profile dimensions
/// - `"apply"`: full-frame, no crop/scale — only speed/LUT/overlay
pub fn render_short(
    backend: &Arc<dyn MediaBackend>,
    config: &AppConfig,
    input_clip: &Path,
    output_dir: &Path,
    profile_name: &str,
    event_metadata: Option<&HashMap<String, String>>,
    overrides: Option<&RenderOverrides>,
    reporter: Option<&ProgressReporter>,
    mode: Option<&str>,
) -> Result<RenderEntry, String> {
    let base_profile = config
        .render_profiles
        .get(profile_name)
        .ok_or_else(|| format!("Render profile '{}' not found", profile_name))?;

    let mut profile = base_profile.clone();
    if let Some(ovr) = overrides {
        apply_overrides(&mut profile, ovr);
    }

    if let Some(r) = reporter {
        r.report("render", 0.0, &format!("Rendering with profile '{profile_name}'"));
    }

    let stem = input_clip
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let output_name = format!("{stem}_{profile_name}.mp4");
    let output = output_dir.join(&output_name);

    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    let plan = if mode == Some("apply") {
        build_apply_plan(input_clip, &output, &profile, config)
    } else {
        build_render_plan(input_clip, &output, &profile, config)
    };

    if let Some(r) = reporter {
        r.report("render", 0.2, "Encoding video");
    }

    let result = backend.render(&plan).map_err(|e| e.to_string())?;

    // If profile has a subtitle_template, composite the overlay
    let final_output = if let Some(ref template_path_str) = profile.subtitle_template {
        if let Some(r) = reporter {
            r.report("overlay", 0.7, "Applying overlay template");
        }
        let template_path = Path::new(template_path_str);
        if template_path.exists() {
            let template = reeln_overlay::template::load_template(template_path)
                .map_err(|e| e.to_string())?;

            let context = event_metadata.cloned().unwrap_or_default();

            let overlay_png = output_dir.join(format!("{stem}_{profile_name}_overlay.png"));
            reeln_overlay::render::render_template_to_png(&template, &context, &overlay_png)
                .map_err(|e| e.to_string())?;

            let composited_output = output_dir.join(format!("{stem}_{profile_name}_final.mp4"));
            let comp_opts = reeln_media::composite::CompositeOptions {
                video_codec: plan.video_codec.clone(),
                crf: plan.crf,
                audio_codec: plan.audio_codec.clone(),
                ..Default::default()
            };
            let comp_result = reeln_media::composite::composite_overlay(
                &result.output,
                &overlay_png,
                &composited_output,
                &comp_opts,
            )
            .map_err(|e| e.to_string())?;

            let _ = std::fs::remove_file(&result.output);
            let _ = std::fs::remove_file(&overlay_png);

            comp_result.output
        } else {
            result.output
        }
    } else {
        result.output
    };

    if let Some(r) = reporter {
        r.report("done", 1.0, "Render complete");
    }

    Ok(RenderEntry {
        input: input_clip.display().to_string(),
        output: final_output.display().to_string(),
        segment_number: 0,
        format: profile_name.to_string(),
        crop_mode: profile.crop_mode.clone().unwrap_or_default(),
        rendered_at: chrono::Utc::now().to_rfc3339(),
        event_id: String::new(),
    })
}

/// Render multiple profiles for one clip (iteration), optionally concatenating.
pub fn render_iteration(
    backend: &Arc<dyn MediaBackend>,
    config: &AppConfig,
    input_clip: &Path,
    output_dir: &Path,
    items: &[IterationItem],
    concat_output: bool,
    reporter: Option<&ProgressReporter>,
    mode: Option<&str>,
    event_metadata: Option<&HashMap<String, String>>,
) -> Result<Vec<RenderEntry>, String> {
    if items.is_empty() {
        return Err("No iteration items provided".to_string());
    }

    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    let total = items.len() as f64;
    let mut entries = Vec::new();

    for (i, item) in items.iter().enumerate() {
        let progress_base = i as f64 / total;
        let progress_end = (i + 1) as f64 / total;

        if let Some(r) = reporter {
            r.report(
                "iteration",
                progress_base,
                &format!("Rendering {}/{} ({})", i + 1, items.len(), item.profile_name),
            );
        }

        let entry = render_short(
            backend,
            config,
            input_clip,
            output_dir,
            &item.profile_name,
            event_metadata,
            item.overrides.as_ref(),
            None, // don't report sub-progress
            mode,
        )?;

        entries.push(entry);

        if let Some(r) = reporter {
            r.report("iteration", progress_end, &format!("Completed {}", item.profile_name));
        }
    }

    // Optionally concatenate all renders into one file
    if concat_output && entries.len() > 1 {
        if let Some(r) = reporter {
            r.report("concat", 0.9, "Concatenating iterations");
        }

        let short_paths: Vec<PathBuf> = entries.iter().map(|e| PathBuf::from(&e.output)).collect();
        let refs: Vec<&Path> = short_paths.iter().map(|p| p.as_path()).collect();

        let stem = input_clip
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("clip");
        let concat_path = output_dir.join(format!("{stem}_iteration.mp4"));

        let opts = ConcatOptions {
            copy: false, // re-encode for smooth transitions
            video_codec: config.video.codec.clone(),
            crf: config.video.crf,
            audio_codec: config.video.audio_codec.clone(),
            audio_rate: 48000,
        };

        backend
            .concat(&refs, &concat_path, &opts)
            .map_err(|e| e.to_string())?;

        // Clean up individual renders
        for path in &short_paths {
            let _ = std::fs::remove_file(path);
        }

        // Return a single entry for the concatenated output
        let profile_names: Vec<&str> = items.iter().map(|i| i.profile_name.as_str()).collect();
        entries = vec![RenderEntry {
            input: input_clip.display().to_string(),
            output: concat_path.display().to_string(),
            segment_number: 0,
            format: profile_names.join("+"),
            crop_mode: String::new(),
            rendered_at: chrono::Utc::now().to_rfc3339(),
            event_id: String::new(),
        }];
    }

    if let Some(r) = reporter {
        r.report("done", 1.0, "Iteration complete");
    }

    Ok(entries)
}

/// Render a low-res preview of a clip.
///
/// When `config` and `profile_name` are provided, applies the profile's crop/scale
/// settings so the preview shows accurate framing. Encoding stays fast (ultrafast, CRF 28).
/// When omitted, falls back to a simple 640p resize.
pub fn render_preview(
    backend: &Arc<dyn MediaBackend>,
    input_clip: &Path,
    output_dir: &Path,
    config: Option<&AppConfig>,
    profile_name: Option<&str>,
    reporter: Option<&ProgressReporter>,
) -> Result<PathBuf, String> {
    let stem = input_clip
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let suffix = profile_name.unwrap_or("preview");
    let output = output_dir.join(format!("{stem}_{suffix}_preview.mp4"));

    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report("render", 0.1, "Rendering preview");
    }

    // Build filters from profile if available, otherwise default 640p
    let filters = match (config, profile_name) {
        (Some(cfg), Some(pname)) => {
            if let Some(profile) = cfg.render_profiles.get(pname) {
                let mut f = Vec::new();
                if let (Some(w), Some(h)) = (profile.width, profile.height) {
                    match profile.crop_mode.as_deref() {
                        Some("crop") => {
                            f.push(format!("crop={w}:{h}"));
                        }
                        Some("pad") => {
                            let pad_color = profile.pad_color.as_deref().unwrap_or("black");
                            f.push(format!(
                                "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color={pad_color}"
                            ));
                        }
                        _ => {
                            f.push(format!("scale={w}:{h}"));
                        }
                    }
                } else if let Some(w) = profile.width {
                    f.push(format!("scale={w}:-2"));
                } else if let Some(h) = profile.height {
                    f.push(format!("scale=-2:{h}"));
                } else {
                    f.push("scale=640:-2".to_string());
                }
                f
            } else {
                vec!["scale=640:-2".to_string()]
            }
        }
        _ => vec!["scale=640:-2".to_string()],
    };

    let plan = RenderPlan {
        input: input_clip.to_path_buf(),
        output: output.clone(),
        video_codec: "libx264".to_string(),
        crf: 28,
        preset: Some("ultrafast".to_string()),
        audio_codec: "aac".to_string(),
        audio_bitrate: Some(96),
        filters,
        filter_complex: None,
        audio_filter: None,
    };

    backend.render(&plan).map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report("done", 1.0, "Preview ready");
    }

    Ok(output)
}

/// Concatenate rendered shorts into a reel.
pub fn render_reel(
    backend: &Arc<dyn MediaBackend>,
    config: &AppConfig,
    shorts: &[PathBuf],
    output: &Path,
    reporter: Option<&ProgressReporter>,
) -> Result<RenderResult, String> {
    if shorts.is_empty() {
        return Err("No shorts to concatenate into a reel".to_string());
    }

    if let Some(r) = reporter {
        r.report("concat", 0.1, &format!("Concatenating {} shorts", shorts.len()));
    }

    let refs: Vec<&Path> = shorts.iter().map(|p| p.as_path()).collect();
    let opts = ConcatOptions {
        copy: true,
        video_codec: config.video.codec.clone(),
        crf: config.video.crf,
        audio_codec: config.video.audio_codec.clone(),
        audio_rate: 48000,
    };

    backend
        .concat(&refs, output, &opts)
        .map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report("done", 1.0, "Reel complete");
    }

    let info = backend.probe(output).map_err(|e| e.to_string())?;

    Ok(RenderResult {
        output: output.to_path_buf(),
        duration_secs: info.duration_secs.unwrap_or(0.0),
    })
}

/// List available render profiles from config.
pub fn list_render_profiles(config: &AppConfig) -> Vec<RenderProfile> {
    config
        .render_profiles
        .iter()
        .map(|(key, profile)| {
            let mut p = profile.clone();
            if p.name.is_empty() {
                p.name = key.clone();
            }
            p
        })
        .collect()
}
