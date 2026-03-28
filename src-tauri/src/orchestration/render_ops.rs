use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use reeln_config::{AppConfig, RenderProfile};
use reeln_media::{ConcatOptions, MediaBackend, RenderPlan, RenderResult};
use reeln_state::RenderEntry;

use super::progress::ProgressReporter;

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
                let pad_color = profile
                    .pad_color
                    .as_deref()
                    .unwrap_or("black");
                filters.push(format!(
                    "scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color={pad_color}"
                ));
            }
            _ => {
                // Default: scale to fit
                filters.push(format!("scale={w}:{h}"));
            }
        }
    } else if let Some(w) = profile.width {
        filters.push(format!("scale={w}:-2"));
    } else if let Some(h) = profile.height {
        filters.push(format!("scale=-2:{h}"));
    }

    // Speed filter
    let mut audio_filter = None;
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

/// Render a short from a clip using a named render profile.
pub fn render_short(
    backend: &Arc<dyn MediaBackend>,
    config: &AppConfig,
    input_clip: &Path,
    output_dir: &Path,
    profile_name: &str,
    event_metadata: Option<&HashMap<String, String>>,
    reporter: Option<&ProgressReporter>,
) -> Result<RenderEntry, String> {
    let profile = config
        .render_profiles
        .get(profile_name)
        .ok_or_else(|| format!("Render profile '{}' not found", profile_name))?;

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

    let plan = build_render_plan(input_clip, &output, profile, config);

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
            let comp_result = reeln_media::composite::composite_overlay_native(
                &result.output,
                &overlay_png,
                &composited_output,
                &comp_opts,
            )
            .map_err(|e| e.to_string())?;

            // Clean up intermediate files
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

/// Render a low-res preview of a clip.
pub fn render_preview(
    backend: &Arc<dyn MediaBackend>,
    input_clip: &Path,
    output_dir: &Path,
    reporter: Option<&ProgressReporter>,
) -> Result<PathBuf, String> {
    let stem = input_clip
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let output = output_dir.join(format!("{stem}_preview.mp4"));

    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report("render", 0.1, "Rendering preview");
    }

    let plan = RenderPlan {
        input: input_clip.to_path_buf(),
        output: output.clone(),
        video_codec: "libx264".to_string(),
        crf: 28,
        preset: Some("ultrafast".to_string()),
        audio_codec: "aac".to_string(),
        audio_bitrate: Some(96),
        filters: vec!["scale=640:-2".to_string()],
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

    // Probe output for duration
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
