use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use reeln_config::{AppConfig, RenderProfile};
use reeln_media::{ConcatOptions, MediaBackend, RenderPlan, RenderResult};
use reeln_state::RenderEntry;
use tauri::AppHandle;

use super::progress::ProgressReporter;

/// Parameters for CLI-based rendering.
///
/// Each field maps 1:1 to a `reeln render short|apply` CLI flag.
/// See CLI-Parity Mandate in CLAUDE.md.
pub struct CliRenderParams<'a> {
    pub cli_path: &'a str,
    pub config_path: Option<&'a str>,
    pub input_clip: &'a Path,
    pub game_dir: &'a Path,
    /// One or more render profiles. When multiple are given, the CLI renders
    /// each and concatenates the outputs into a single file — handling queue
    /// entry creation (`--queue`) and `game.json` bookkeeping in one place.
    /// Maps to one or more `--render-profile` flags.
    pub profile_names: &'a [&'a str],
    pub event_id: Option<&'a str>,
    pub mode: Option<&'a str>,
    pub overrides: Option<&'a RenderOverrides>,
    pub scorer: Option<&'a str>,
    pub assist1: Option<&'a str>,
    pub assist2: Option<&'a str>,
    /// Jersey numbers for scorer[,assist1[,assist2]] — maps to `--player-numbers`.
    pub player_numbers: Option<&'a str>,
    pub iterate: bool,
    pub event_type: Option<&'a str>,
    pub debug: bool,
    /// Disable branding overlay — maps to `--no-branding`. Default false (branding ON).
    pub no_branding: bool,
    /// Explicit output file path — maps to `--output`. When None, CLI uses its default.
    pub output_path: Option<&'a Path>,
    /// Add to render queue instead of immediate publish — maps to `--queue`.
    pub queue: bool,
    /// Optional Tauri app handle for emitting log events to the frontend.
    pub app_handle: Option<&'a AppHandle>,
}

impl<'a> CliRenderParams<'a> {
    /// First profile name, for fallback output detection. Empty string when
    /// no profiles were provided (which callers should treat as an error).
    fn first_profile(&self) -> &'a str {
        self.profile_names.first().copied().unwrap_or("")
    }
}

/// Render via the reeln CLI subprocess. Returns new RenderEntries added to game state.
///
/// The CLI handles: builtin templates, speed_segments, smart zoom, player overlays,
/// POST_RENDER plugin hooks, and all other features the native backend can't do.
pub fn render_via_cli(params: &CliRenderParams) -> Result<Vec<RenderEntry>, String> {
    if params.profile_names.is_empty() {
        return Err("render_via_cli called with no profile names".to_string());
    }

    // Snapshot renders before CLI call so we can diff afterward
    let pre_render_count = reeln_state::load_game_state(params.game_dir)
        .map(|s| s.renders.len())
        .unwrap_or(0);

    // Build the CLI command
    let subcommand = if params.mode == Some("apply") {
        "apply"
    } else {
        "short"
    };
    let mut cmd = Command::new(params.cli_path);
    cmd.arg("render").arg(subcommand);

    // Positional: clip path
    cmd.arg(params.input_clip);

    // Profiles — repeatable. When multiple are given, the CLI renders each
    // profile and concatenates the results into one output file, and (with
    // --queue) creates exactly one queue entry for the concatenated file.
    for name in params.profile_names {
        cmd.arg("--render-profile").arg(name);
    }

    // Game dir
    cmd.arg("--game-dir").arg(params.game_dir);

    // Explicit output path (avoids filename collision between profiles)
    if let Some(out) = params.output_path {
        cmd.arg("--output").arg(out);
    }

    // Event ID
    if let Some(eid) = params.event_id
        && !eid.is_empty()
    {
        cmd.arg("--event").arg(eid);
    }

    // Config path
    if let Some(config) = params.config_path {
        cmd.arg("--config").arg(config);
    }

    // Overrides (only for "short" mode — "apply" doesn't accept these)
    if subcommand == "short"
        && let Some(ovr) = params.overrides
    {
        if let Some(ref crop) = ovr.crop_mode
            && !crop.is_empty()
        {
            cmd.arg("--crop").arg(crop);
        }
        if let Some(scale) = ovr.scale
            && (scale - 1.0).abs() > f64::EPSILON
        {
            cmd.arg("--scale").arg(scale.to_string());
        }
        if let Some(speed) = ovr.speed
            && (speed - 1.0).abs() > f64::EPSILON
        {
            cmd.arg("--speed").arg(speed.to_string());
        }
        if ovr.smart == Some(true) {
            cmd.arg("--smart");
        }
        if let Some(zf) = ovr.zoom_frames {
            cmd.arg("--zoom-frames").arg(zf.to_string());
        }
        if let Some(ref pc) = ovr.pad_color
            && !pc.is_empty()
        {
            cmd.arg("--pad-color").arg(pc);
        }
    }

    // Anchor (crop anchor position) — only for "short" mode
    if subcommand == "short"
        && let Some(ovr) = params.overrides
        && let (Some(ax), Some(ay)) = (ovr.anchor_x, ovr.anchor_y)
    {
        cmd.arg("--anchor").arg(format!("{ax},{ay}"));
    }

    // Plugin-contributed fields → --plugin-input KEY=VALUE
    if let Some(ovr) = params.overrides {
        for (key, val) in &ovr.extra {
            let val_str = match val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            cmd.arg("--plugin-input").arg(format!("{key}={val_str}"));
        }
    }

    // Player info
    if let Some(scorer) = params.scorer
        && !scorer.is_empty()
    {
        cmd.arg("--player").arg(scorer);
    }
    if params.assist1.is_some() || params.assist2.is_some() {
        let assists: Vec<&str> = [params.assist1, params.assist2]
            .iter()
            .filter_map(|a| a.filter(|s| !s.is_empty()))
            .collect();
        if !assists.is_empty() {
            cmd.arg("--assists").arg(assists.join(","));
        }
    }
    // Jersey numbers for roster lookup
    if let Some(pn) = params.player_numbers
        && !pn.is_empty()
    {
        cmd.arg("--player-numbers").arg(pn);
    }

    // Event type (for scoring team resolution)
    if let Some(et) = params.event_type
        && !et.is_empty()
    {
        cmd.arg("--event-type").arg(et);
    }

    // Iterate flag
    if params.iterate {
        cmd.arg("--iterate");
    }

    // Debug artifacts
    if params.debug {
        cmd.arg("--debug");
    }

    // Branding: on by default (matches CLI default). Only disable when explicitly requested.
    if params.no_branding {
        cmd.arg("--no-branding");
    }

    // Queue: add to render queue instead of immediate publish
    if params.queue {
        cmd.arg("--queue");
    }

    // Log the CLI command before execution
    if let Some(app) = params.app_handle {
        // Build a displayable args list from the Command
        let args_display: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        let full_cmd = format!(
            "{} {}",
            cmd.get_program().to_string_lossy(),
            args_display.join(" ")
        );
        crate::dock_log::emit(app, "info", "Render", &format!("$ {full_cmd}"));
    }

    // Execute
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to execute reeln CLI: {e}"))?;

    // Log CLI output
    if let Some(app) = params.app_handle {
        crate::dock_log::log_cli_output(app, "Render", &output);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        return Err(format!(
            "reeln render {} failed (exit code {code}):\n{stderr}",
            subcommand
        ));
    }

    // CLI succeeded — read back game state to find new render entries
    let post_state = reeln_state::load_game_state(params.game_dir)
        .map_err(|e| format!("Failed to read game state after render: {e}"))?;

    let new_entries: Vec<RenderEntry> = post_state
        .renders
        .into_iter()
        .skip(pre_render_count)
        .collect();

    // If CLI didn't record entries (iteration path doesn't save to game.json),
    // parse the CLI output to find the result file, or search common locations
    if new_entries.is_empty() {
        let _stdout = String::from_utf8_lossy(&output.stdout);
        let stem = params
            .input_clip
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("clip");
        let clip_parent = params.input_clip.parent().unwrap_or(params.game_dir);

        // Try to find "Output: <path>" in stderr (CLI logs to stderr)
        let output_from_log = stderr
            .lines()
            .find(|l| l.starts_with("Output: "))
            .map(|l| PathBuf::from(l.trim_start_matches("Output: ").trim()));

        // Search common output locations
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Some(ref p) = output_from_log {
            candidates.push(p.clone());
        }
        // CLI short mode: {clip_parent}/shorts/{stem}_short.mp4
        candidates.push(clip_parent.join("shorts").join(format!("{stem}_short.mp4")));
        // CLI iteration: {clip_parent}/shorts/{stem}_short.mp4 (same)
        candidates.push(
            params
                .game_dir
                .join("renders")
                .join(format!("{stem}_iteration.mp4")),
        );
        candidates.push(
            params
                .game_dir
                .join("renders")
                .join(format!("{stem}_{}.mp4", params.first_profile())),
        );

        if let Some(output_path) = candidates.iter().find(|p| p.is_file()) {
            // Stamp the entry's format with the full profile list ("a+b+c")
            // for multi-profile iterations, matching reeln-core's IterationResult.
            let format = params.profile_names.join("+");
            let entry = RenderEntry {
                input: params.input_clip.display().to_string(),
                output: output_path.display().to_string(),
                segment_number: 0,
                format,
                crop_mode: params
                    .overrides
                    .and_then(|o| o.crop_mode.clone())
                    .unwrap_or_default(),
                rendered_at: chrono::Utc::now().to_rfc3339(),
                event_id: params.event_id.unwrap_or("").to_string(),
            };
            // Save to game state
            let mut state =
                reeln_state::load_game_state(params.game_dir).map_err(|e| e.to_string())?;
            reeln_state::add_render(&mut state, entry.clone());
            reeln_state::save_game_state(&state, params.game_dir).map_err(|e| e.to_string())?;
            return Ok(vec![entry]);
        }
    }

    Ok(new_entries)
}

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
    /// Plugin-contributed fields — passed to CLI as `--plugin-input KEY=VALUE`.
    pub extra: HashMap<String, serde_json::Value>,
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

    // speed_segments: apply slowest segment's speed as uniform approximation.
    // Proper variable speed requires the CLI bridge.
    let effective_speed = profile
        .speed_segments
        .as_ref()
        .and_then(|segs| {
            segs.iter()
                .map(|s| s.speed)
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        })
        .or(profile.speed);
    if let Some(speed) = effective_speed
        && (speed - 1.0).abs() > f64::EPSILON
    {
        let pts_factor = 1.0 / speed;
        filters.push(format!("setpts={pts_factor:.4}*PTS"));
        audio_filter = Some(format!("atempo={speed}"));
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

    // Speed filter.
    // NOTE: speed_segments (variable speed per time range) requires the CLI
    // bridge — the native libav backend can't handle multi-segment trim+concat.
    // When speed_segments is set, we apply the slowest segment's speed as a
    // uniform approximation. Use the CLI for proper variable speed rendering.
    let mut audio_filter = None;
    let effective_speed = profile
        .speed_segments
        .as_ref()
        .and_then(|segs| {
            segs.iter()
                .map(|s| s.speed)
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        })
        .or(profile.speed);
    if let Some(speed) = effective_speed
        && (speed - 1.0).abs() > f64::EPSILON
    {
        let pts_factor = 1.0 / speed;
        filters.push(format!("setpts={pts_factor:.4}*PTS"));
        audio_filter = Some(format!("atempo={speed}"));
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
#[allow(clippy::too_many_arguments)]
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
        r.report(
            "render",
            0.0,
            &format!("Rendering with profile '{profile_name}'"),
        );
    }

    let stem = input_clip
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let output_name = format!("{stem}_{profile_name}.mp4");
    let output = output_dir.join(&output_name);

    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report("render", 0.2, "Encoding video");
    }

    let plan = if mode == Some("apply") {
        build_apply_plan(input_clip, &output, &profile, config)
    } else {
        build_render_plan(input_clip, &output, &profile, config)
    };
    let result = backend.render(&plan).map_err(|e| e.to_string())?;

    // If profile has a subtitle_template, composite the overlay.
    // Skip "builtin:" templates — these require the CLI to resolve and render.
    // The dock's native backend handles only file-path templates.
    let final_output = if let Some(ref template_path_str) = profile.subtitle_template {
        if template_path_str.starts_with("builtin:") {
            // Builtin templates need the CLI bridge — skip overlay in native mode
            result.output
        } else {
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
                    video_codec: profile
                        .codec
                        .clone()
                        .unwrap_or_else(|| config.video.codec.clone()),
                    crf: profile.crf.unwrap_or(config.video.crf),
                    audio_codec: profile
                        .audio_codec
                        .clone()
                        .unwrap_or_else(|| config.video.audio_codec.clone()),
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
#[allow(clippy::too_many_arguments)]
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
                &format!(
                    "Rendering {}/{} ({})",
                    i + 1,
                    items.len(),
                    item.profile_name
                ),
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
            r.report(
                "iteration",
                progress_end,
                &format!("Completed {}", item.profile_name),
            );
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
        r.report(
            "concat",
            0.1,
            &format!("Concatenating {} shorts", shorts.len()),
        );
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

/// Render a preview from an inline (unsaved) profile JSON.
/// Used by the profile editor for real-time preview.
///
/// Safety: probes the input first so filter dimensions never exceed the source,
/// preventing libav crashes from impossible crop/scale combos.
pub fn render_profile_preview(
    backend: &Arc<dyn MediaBackend>,
    input_clip: &Path,
    output_dir: &Path,
    config: Option<&AppConfig>,
    profile_json: &serde_json::Value,
    reporter: Option<&ProgressReporter>,
) -> Result<PathBuf, String> {
    if !input_clip.is_file() {
        return Err(format!("Clip not found: {}", input_clip.display()));
    }

    let profile: RenderProfile =
        serde_json::from_value(profile_json.clone()).map_err(|e| e.to_string())?;

    // Probe source to get dimensions — needed for safe filter construction
    let source = backend.probe(input_clip).map_err(|e| e.to_string())?;
    let src_w = source.width.unwrap_or(1920);
    let _src_h = source.height.unwrap_or(1080);

    let stem = input_clip
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let output = output_dir.join(format!("{stem}_profile_preview.mp4"));

    std::fs::create_dir_all(output_dir).map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report("render", 0.1, "Rendering profile preview");
    }

    let mut filters = Vec::new();

    // Scale/crop — always safe: scale first, then crop if needed
    if let (Some(w), Some(h)) = (profile.width, profile.height) {
        // Ensure even dimensions
        let w = w & !1;
        let h = h & !1;
        match profile.crop_mode.as_deref() {
            Some("crop") => {
                // Scale so the smaller dimension fills the target, then center-crop
                // This avoids crop=WxH on a source that's smaller than WxH.
                filters.push(format!(
                    "scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}"
                ));
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
        let w = w & !1;
        filters.push(format!("scale={w}:-2"));
    } else if let Some(h) = profile.height {
        let h = h & !1;
        filters.push(format!("scale=-2:{h}"));
    } else {
        // No dimensions — scale down for fast preview, keep aspect
        let preview_w = src_w.min(640) & !1;
        filters.push(format!("scale={preview_w}:-2"));
    }

    // Speed filter
    let mut audio_filter = None;
    if let Some(speed) = profile.speed
        && (speed - 1.0).abs() > f64::EPSILON
        && speed > 0.0
    {
        let pts_factor = 1.0 / speed;
        filters.push(format!("setpts={pts_factor:.4}*PTS"));
        // atempo only accepts 0.5–100.0
        let clamped = speed.clamp(0.5, 100.0);
        audio_filter = Some(format!("atempo={clamped}"));
    }

    // LUT filter — only if file exists
    if let Some(ref lut) = profile.lut
        && !lut.is_empty()
        && Path::new(lut).is_file()
    {
        filters.push(format!("lut3d={lut}"));
    }

    let video_codec = profile.codec.clone().unwrap_or_else(|| {
        config
            .map(|c| c.video.codec.clone())
            .unwrap_or_else(|| "libx264".to_string())
    });

    let plan = RenderPlan {
        input: input_clip.to_path_buf(),
        output: output.clone(),
        video_codec,
        crf: 28,
        preset: Some("ultrafast".to_string()),
        audio_codec: "aac".to_string(),
        audio_bitrate: Some(96),
        filters,
        filter_complex: None,
        audio_filter,
    };

    backend.render(&plan).map_err(|e| e.to_string())?;

    if let Some(r) = reporter {
        r.report("done", 1.0, "Profile preview ready");
    }

    Ok(output)
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

// ── Preview proxy ──────────────────────────────────────────────────

/// File extensions that HTML5 `<video>` can play natively.
const WEB_PLAYABLE_EXTENSIONS: &[&str] = &["mp4", "mov", "webm"];

/// Generate an MP4 proxy for non-web video formats (MKV, AVI, TS, FLV).
///
/// Returns the original path unchanged if the file is already web-playable.
/// Uses remux (stream copy) when the source has H.264 video, or falls back
/// to full transcode with ultrafast/CRF 28 settings.
///
/// Proxy files are cached in `proxy_dir` by a hash of the source path.
pub fn prepare_preview_proxy(
    backend: &Arc<dyn MediaBackend>,
    input_clip: &Path,
    proxy_dir: &Path,
) -> Result<PathBuf, String> {
    // 1. Already web-playable? Return as-is.
    let ext = input_clip
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if WEB_PLAYABLE_EXTENSIONS.contains(&ext.as_str()) {
        return Ok(input_clip.to_path_buf());
    }

    // 2. Compute deterministic proxy path from source path hash.
    let stem = input_clip
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("clip");
    let path_hash = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        input_clip.to_string_lossy().as_ref().hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };
    let proxy_path = proxy_dir.join(format!("{path_hash}_{stem}.mp4"));

    // 3. Cache hit — proxy already exists and is non-empty.
    if proxy_path.is_file()
        && let Ok(meta) = std::fs::metadata(&proxy_path)
        && meta.len() > 0
    {
        return Ok(proxy_path);
    }

    // 4. Ensure proxy directory exists.
    std::fs::create_dir_all(proxy_dir).map_err(|e| e.to_string())?;

    // 5. Probe source to determine codec.
    let info = backend.probe(input_clip).map_err(|e| e.to_string())?;
    let codec = info.codec.as_deref().unwrap_or("");

    // 6. Try remux (stream copy) for H.264 sources — near-instant.
    if codec == "h264" {
        let opts = ConcatOptions {
            copy: true,
            video_codec: String::new(),
            crf: 0,
            audio_codec: String::new(),
            audio_rate: 0,
        };
        let remux_result = backend.concat(&[input_clip], &proxy_path, &opts);
        if remux_result.is_ok() {
            return Ok(proxy_path);
        }
        // Remux failed (e.g., incompatible audio codec) — clean up partial file
        let _ = std::fs::remove_file(&proxy_path);
    }

    // 7. Full transcode — preserves original resolution.
    let plan = RenderPlan {
        input: input_clip.to_path_buf(),
        output: proxy_path.clone(),
        video_codec: "libx264".to_string(),
        crf: 28,
        preset: Some("ultrafast".to_string()),
        audio_codec: "aac".to_string(),
        audio_bitrate: Some(96),
        filters: vec![],
        filter_complex: None,
        audio_filter: None,
    };

    backend.render(&plan).map_err(|e| e.to_string())?;

    Ok(proxy_path)
}

/// Remove proxy files older than `max_age` from the proxy cache directory.
pub fn cleanup_proxy_cache(proxy_dir: &Path, max_age: std::time::Duration) {
    let entries = match std::fs::read_dir(proxy_dir) {
        Ok(e) => e,
        Err(_) => return, // Directory doesn't exist yet — nothing to clean
    };

    let cutoff = std::time::SystemTime::now() - max_age;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Ok(meta) = path.metadata()
            && let Ok(modified) = meta.modified()
            && modified < cutoff
        {
            let _ = std::fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Create a script that dumps its CLI args to a file, then exits with code 1
    /// so render_via_cli returns early (before trying to load game state post-render).
    fn make_arg_dump_script(dir: &Path, args_file: &Path) -> PathBuf {
        let script = dir.join("fake_reeln.sh");
        let mut f = std::fs::File::create(&script).unwrap();
        writeln!(f, "#!/bin/sh").unwrap();
        writeln!(f, "printf '%s\\n' \"$@\" > \"{}\"", args_file.display()).unwrap();
        writeln!(f, "echo 'test failure' >&2").unwrap();
        writeln!(f, "exit 1").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    }

    fn read_dumped_args(args_file: &Path) -> Vec<String> {
        std::fs::read_to_string(args_file)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect()
    }

    // -----------------------------------------------------------------------
    // render_via_cli — CLI arg construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_cli_args_short_mode_minimal() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: None,
            mode: None, // defaults to "short"
            overrides: None,
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params); // will error — we only care about args
        let args = read_dumped_args(&args_file);

        assert_eq!(args[0], "render");
        assert_eq!(args[1], "short");
        assert_eq!(args[2], input.to_str().unwrap());
        assert_eq!(args[3], "--render-profile");
        assert_eq!(args[4], "tiktok");
        assert_eq!(args[5], "--game-dir");
        assert_eq!(args[6], game_dir.to_str().unwrap());
        // --no-branding only when explicitly requested (not hardcoded)
        assert!(!args.contains(&"--no-branding".to_string()));
        // No --config, --event, --iterate, --debug, --player-numbers
        assert!(!args.contains(&"--config".to_string()));
        assert!(!args.contains(&"--event".to_string()));
        assert!(!args.contains(&"--iterate".to_string()));
        assert!(!args.contains(&"--debug".to_string()));
        assert!(!args.contains(&"--player-numbers".to_string()));
    }

    #[test]
    fn test_cli_args_apply_mode() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["full"],
            event_id: None,
            mode: Some("apply"),
            overrides: None,
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        assert_eq!(args[0], "render");
        assert_eq!(args[1], "apply");
        // --no-branding should NOT be present for apply mode
        assert!(!args.contains(&"--no-branding".to_string()));
    }

    #[test]
    fn test_cli_args_with_config_path() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: Some("/config/google-test.json"),
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: None,
            mode: None,
            overrides: None,
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        let idx = args.iter().position(|a| a == "--config").unwrap();
        assert_eq!(args[idx + 1], "/config/google-test.json");
    }

    #[test]
    fn test_cli_args_with_output_path() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();
        let out = game_dir.join("shorts").join("clip_player_short.mp4");

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: None,
            mode: None,
            overrides: None,
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: Some(out.as_path()),
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        let idx = args.iter().position(|a| a == "--output").unwrap();
        assert_eq!(args[idx + 1], out.to_str().unwrap());
    }

    #[test]
    fn test_cli_args_with_event_id() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: Some("goal_1"),
            mode: None,
            overrides: None,
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        let idx = args.iter().position(|a| a == "--event").unwrap();
        assert_eq!(args[idx + 1], "goal_1");
    }

    #[test]
    fn test_cli_args_empty_event_id_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: Some(""),
            mode: None,
            overrides: None,
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);
        assert!(!args.contains(&"--event".to_string()));
    }

    #[test]
    fn test_cli_args_overrides_short_mode() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let overrides = RenderOverrides {
            crop_mode: Some("pad".to_string()),
            scale: Some(0.5),
            speed: Some(0.75),
            smart: Some(true),
            zoom_frames: Some(15),
            pad_color: Some("white".to_string()),
            ..Default::default()
        };

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: None,
            mode: None,
            overrides: Some(&overrides),
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        let crop_idx = args.iter().position(|a| a == "--crop").unwrap();
        assert_eq!(args[crop_idx + 1], "pad");

        let scale_idx = args.iter().position(|a| a == "--scale").unwrap();
        assert_eq!(args[scale_idx + 1], "0.5");

        let speed_idx = args.iter().position(|a| a == "--speed").unwrap();
        assert_eq!(args[speed_idx + 1], "0.75");

        assert!(args.contains(&"--smart".to_string()));

        let zf_idx = args.iter().position(|a| a == "--zoom-frames").unwrap();
        assert_eq!(args[zf_idx + 1], "15");

        let pc_idx = args.iter().position(|a| a == "--pad-color").unwrap();
        assert_eq!(args[pc_idx + 1], "white");
    }

    #[test]
    fn test_cli_args_overrides_skipped_in_apply_mode() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let overrides = RenderOverrides {
            crop_mode: Some("pad".to_string()),
            scale: Some(0.5),
            smart: Some(true),
            ..Default::default()
        };

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["full"],
            event_id: None,
            mode: Some("apply"),
            overrides: Some(&overrides),
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        // Overrides should NOT be passed in apply mode
        assert!(!args.contains(&"--crop".to_string()));
        assert!(!args.contains(&"--scale".to_string()));
        assert!(!args.contains(&"--smart".to_string()));
    }

    #[test]
    fn test_cli_args_scale_1_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let overrides = RenderOverrides {
            scale: Some(1.0), // should be skipped
            speed: Some(1.0), // should be skipped
            ..Default::default()
        };

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: None,
            mode: None,
            overrides: Some(&overrides),
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        assert!(!args.contains(&"--scale".to_string()));
        assert!(!args.contains(&"--speed".to_string()));
    }

    #[test]
    fn test_cli_args_scorer_and_assists() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: None,
            mode: None,
            overrides: None,
            scorer: Some("Player One"),
            assist1: Some("Player Two"),
            assist2: Some("Player Three"),
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        let player_idx = args.iter().position(|a| a == "--player").unwrap();
        assert_eq!(args[player_idx + 1], "Player One");

        let assists_idx = args.iter().position(|a| a == "--assists").unwrap();
        assert_eq!(args[assists_idx + 1], "Player Two,Player Three");
    }

    #[test]
    fn test_cli_args_iterate_and_debug() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: None,
            mode: None,
            overrides: None,
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: true,
            event_type: Some("goal"),
            debug: true,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        assert!(args.contains(&"--iterate".to_string()));
        assert!(args.contains(&"--debug".to_string()));
        // no_branding is false — --no-branding should NOT be present
        assert!(!args.contains(&"--no-branding".to_string()));

        let et_idx = args.iter().position(|a| a == "--event-type").unwrap();
        assert_eq!(args[et_idx + 1], "goal");
    }

    #[test]
    fn test_cli_args_all_params_combined() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let overrides = RenderOverrides {
            crop_mode: Some("crop".to_string()),
            scale: Some(0.8),
            speed: Some(0.5),
            smart: Some(true),
            zoom_frames: Some(10),
            pad_color: Some("black".to_string()),
            extra: {
                let mut m = HashMap::new();
                m.insert("smart_zoom".to_string(), serde_json::Value::Bool(true));
                m.insert(
                    "quality".to_string(),
                    serde_json::Value::String("high".to_string()),
                );
                m
            },
            ..Default::default()
        };

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: Some("/config/google.json"),
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["tiktok"],
            event_id: Some("goal_1"),
            mode: None,
            overrides: Some(&overrides),
            scorer: Some("Scorer"),
            assist1: Some("A1"),
            assist2: Some("A2"),
            iterate: true,
            event_type: Some("goal"),
            debug: true,
            player_numbers: Some("48,3,58"),
            no_branding: true,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        // Verify all args are present
        assert_eq!(args[0], "render");
        assert_eq!(args[1], "short");
        assert!(args.contains(&"--config".to_string()));
        assert!(args.contains(&"--event".to_string()));
        assert!(args.contains(&"--crop".to_string()));
        assert!(args.contains(&"--scale".to_string()));
        assert!(args.contains(&"--speed".to_string()));
        assert!(args.contains(&"--smart".to_string()));
        assert!(args.contains(&"--zoom-frames".to_string()));
        assert!(args.contains(&"--pad-color".to_string()));
        assert!(args.contains(&"--player".to_string()));
        assert!(args.contains(&"--assists".to_string()));
        assert!(args.contains(&"--iterate".to_string()));
        assert!(args.contains(&"--event-type".to_string()));
        assert!(args.contains(&"--debug".to_string()));
        assert!(args.contains(&"--no-branding".to_string()));
        let pn_idx = args.iter().position(|a| a == "--player-numbers").unwrap();
        assert_eq!(args[pn_idx + 1], "48,3,58");
        // Plugin inputs
        let pi_indices: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "--plugin-input")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(pi_indices.len(), 2);
        let pi_values: Vec<&str> = pi_indices.iter().map(|i| args[i + 1].as_str()).collect();
        assert!(pi_values.contains(&"smart_zoom=true"));
        assert!(pi_values.contains(&"quality=high"));
    }

    #[test]
    fn test_cli_args_multi_profile_emits_repeatable_render_profile_flags() {
        // Core contract for the "one render, one queue entry" fix:
        // when profile_names contains ≥2 names, render_via_cli emits
        // --render-profile once per name, in order. The CLI takes it from
        // there (renders each, concatenates, queues exactly once).
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &["player-overlay", "slowmo-ten-second-clip"],
            event_id: None,
            mode: None,
            overrides: None,
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false, // NOT --iterate — we pass an explicit list instead
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: true,
            app_handle: None,
        };

        let _ = render_via_cli(&params);
        let args = read_dumped_args(&args_file);

        // Every name should appear exactly once after a --render-profile flag,
        // and in the same order the caller provided.
        let rp_indices: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "--render-profile")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            rp_indices.len(),
            2,
            "expected two --render-profile flags; got args: {args:?}"
        );
        assert_eq!(args[rp_indices[0] + 1], "player-overlay");
        assert_eq!(args[rp_indices[1] + 1], "slowmo-ten-second-clip");

        // --queue must be forwarded so the CLI creates the single queue
        // entry for the concatenated output.
        assert!(args.contains(&"--queue".to_string()));
        // --iterate must NOT be emitted — we're providing the list explicitly.
        assert!(!args.contains(&"--iterate".to_string()));
    }

    #[test]
    fn test_render_via_cli_rejects_empty_profile_list() {
        // Defensive: an empty profile list is a caller bug and should error
        // loudly instead of silently invoking the CLI with no profile.
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let game_dir = dir.path().join("game");
        std::fs::create_dir_all(&game_dir).unwrap();

        let params = CliRenderParams {
            cli_path: script.to_str().unwrap(),
            config_path: None,
            input_clip: &input,
            game_dir: &game_dir,
            profile_names: &[],
            event_id: None,
            mode: None,
            overrides: None,
            scorer: None,
            assist1: None,
            assist2: None,
            iterate: false,
            event_type: None,
            debug: false,
            player_numbers: None,
            no_branding: false,
            output_path: None,
            queue: false,
            app_handle: None,
        };

        let result = render_via_cli(&params);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no profile names"));
        // The script should not have been invoked either.
        assert!(!args_file.is_file());
    }

    // -----------------------------------------------------------------------
    // apply_overrides
    // -----------------------------------------------------------------------

    #[test]
    fn test_apply_overrides_sets_all_fields() {
        let mut profile = serde_json::from_str::<RenderProfile>("{}").unwrap();
        let overrides = RenderOverrides {
            crop_mode: Some("pad".to_string()),
            scale: Some(0.5),
            speed: Some(2.0),
            smart: Some(true),
            anchor_x: Some(0.3),
            anchor_y: Some(0.7),
            pad_color: Some("white".to_string()),
            zoom_frames: None, // not applied in apply_overrides (only CLI flag)
            extra: HashMap::new(),
        };

        apply_overrides(&mut profile, &overrides);

        assert_eq!(profile.crop_mode, Some("pad".to_string()));
        assert_eq!(profile.scale, Some(0.5));
        assert_eq!(profile.speed, Some(2.0));
        assert_eq!(profile.smart, Some(true));
        assert_eq!(profile.anchor_x, Some(0.3));
        assert_eq!(profile.anchor_y, Some(0.7));
        assert_eq!(profile.pad_color, Some("white".to_string()));
    }

    #[test]
    fn test_apply_overrides_none_preserves_existing() {
        let mut profile = serde_json::from_str::<RenderProfile>("{}").unwrap();
        profile.crop_mode = Some("crop".to_string());
        profile.speed = Some(1.5);

        let overrides = RenderOverrides::default(); // all None

        apply_overrides(&mut profile, &overrides);

        assert_eq!(profile.crop_mode, Some("crop".to_string()));
        assert_eq!(profile.speed, Some(1.5));
    }

    // -----------------------------------------------------------------------
    // build_render_plan — filter construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_render_plan_width_and_height_scale() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "width": 1080,
            "height": 1920
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        assert!(plan.filters.contains(&"scale=1080:1920".to_string()));
    }

    #[test]
    fn test_build_render_plan_crop_mode() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "width": 1080,
            "height": 1920,
            "crop_mode": "crop"
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        assert!(plan.filters.contains(&"crop=1080:1920".to_string()));
        // Should NOT have a scale filter
        assert!(!plan.filters.iter().any(|f| f.starts_with("scale=")));
    }

    #[test]
    fn test_build_render_plan_pad_mode() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "width": 1080,
            "height": 1920,
            "crop_mode": "pad",
            "pad_color": "red"
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        let pad_filter = plan.filters.iter().find(|f| f.contains("pad=")).unwrap();
        assert!(pad_filter.contains("color=red"));
        assert!(pad_filter.contains("scale=1080:1920:force_original_aspect_ratio=decrease"));
    }

    #[test]
    fn test_build_render_plan_pad_mode_default_color() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "width": 1080,
            "height": 1920,
            "crop_mode": "pad"
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        let pad_filter = plan.filters.iter().find(|f| f.contains("pad=")).unwrap();
        assert!(pad_filter.contains("color=black"));
    }

    #[test]
    fn test_build_render_plan_width_only() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "width": 720
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        assert!(plan.filters.contains(&"scale=720:-2".to_string()));
    }

    #[test]
    fn test_build_render_plan_height_only() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "height": 480
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        assert!(plan.filters.contains(&"scale=-2:480".to_string()));
    }

    #[test]
    fn test_build_render_plan_speed_not_1() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "speed": 0.5
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        // pts_factor = 1.0 / 0.5 = 2.0
        let setpts = plan
            .filters
            .iter()
            .find(|f| f.starts_with("setpts="))
            .unwrap();
        assert!(setpts.contains("2.0000"));
        assert_eq!(plan.audio_filter, Some("atempo=0.5".to_string()));
    }

    #[test]
    fn test_build_render_plan_speed_1_no_filter() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "speed": 1.0
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        assert!(!plan.filters.iter().any(|f| f.starts_with("setpts=")));
        assert!(plan.audio_filter.is_none());
    }

    #[test]
    fn test_build_render_plan_lut_filter() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "lut": "/path/to/my.cube"
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        assert!(plan.filters.contains(&"lut3d=/path/to/my.cube".to_string()));
    }

    #[test]
    fn test_build_render_plan_speed_segments_uses_minimum() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "speed_segments": [
                {"speed": 0.8, "until": 5.0},
                {"speed": 0.3, "until": 10.0},
                {"speed": 1.5}
            ]
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_render_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        // Minimum speed is 0.3, pts_factor = 1.0 / 0.3 = 3.3333
        let setpts = plan
            .filters
            .iter()
            .find(|f| f.starts_with("setpts="))
            .unwrap();
        assert!(setpts.contains("3.3333"));
        assert_eq!(plan.audio_filter, Some("atempo=0.3".to_string()));
    }

    // -----------------------------------------------------------------------
    // build_apply_plan — full-frame (no crop/scale)
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_apply_plan_no_scale_or_crop() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "width": 1080,
            "height": 1920,
            "crop_mode": "crop"
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_apply_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        // Apply plan should have NO scale or crop filters
        assert!(
            !plan
                .filters
                .iter()
                .any(|f| f.starts_with("scale=") || f.starts_with("crop="))
        );
    }

    #[test]
    fn test_build_apply_plan_speed_applies() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "speed": 2.0
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_apply_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        let setpts = plan
            .filters
            .iter()
            .find(|f| f.starts_with("setpts="))
            .unwrap();
        assert!(setpts.contains("0.5000"));
        assert_eq!(plan.audio_filter, Some("atempo=2".to_string()));
    }

    #[test]
    fn test_build_apply_plan_lut_applies() {
        let profile: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "test",
            "lut": "/luts/warm.cube"
        }))
        .unwrap();
        let config = AppConfig::default();
        let plan = build_apply_plan(
            Path::new("/input.mp4"),
            Path::new("/output.mp4"),
            &profile,
            &config,
        );
        assert!(plan.filters.contains(&"lut3d=/luts/warm.cube".to_string()));
    }

    // -----------------------------------------------------------------------
    // render_short — mock backend
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_short_basic() {
        let backend = crate::test_utils::mock_backend();
        let config = {
            let mut c = AppConfig::default();
            let profile: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "tiktok",
                "width": 1080,
                "height": 1920
            }))
            .unwrap();
            c.render_profiles.insert("tiktok".to_string(), profile);
            c
        };
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("renders");

        let result = render_short(
            &backend,
            &config,
            &input,
            &output_dir,
            "tiktok",
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert!(entry.output.contains("tiktok"));
        assert_eq!(entry.format, "tiktok");
        assert!(Path::new(&entry.output).exists());
    }

    #[test]
    fn test_render_short_apply_mode() {
        let backend = crate::test_utils::mock_backend();
        let config = {
            let mut c = AppConfig::default();
            let profile: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "full",
                "width": 1920,
                "height": 1080
            }))
            .unwrap();
            c.render_profiles.insert("full".to_string(), profile);
            c
        };
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("renders");

        let result = render_short(
            &backend,
            &config,
            &input,
            &output_dir,
            "full",
            None,
            None,
            None,
            Some("apply"),
        );
        assert!(result.is_ok());
        let entry = result.unwrap();
        assert_eq!(entry.format, "full");
    }

    #[test]
    fn test_render_short_with_overrides() {
        let backend = crate::test_utils::mock_backend();
        let config = {
            let mut c = AppConfig::default();
            let profile: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "tiktok",
                "width": 1080,
                "height": 1920
            }))
            .unwrap();
            c.render_profiles.insert("tiktok".to_string(), profile);
            c
        };
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("renders");

        let overrides = RenderOverrides {
            crop_mode: Some("crop".to_string()),
            speed: Some(0.5),
            ..Default::default()
        };

        let result = render_short(
            &backend,
            &config,
            &input,
            &output_dir,
            "tiktok",
            None,
            Some(&overrides),
            None,
            None,
        );
        assert!(result.is_ok());
        let entry = result.unwrap();
        // Overrides applied: crop_mode should be reflected in entry
        assert_eq!(entry.crop_mode, "crop");
    }

    #[test]
    fn test_render_short_missing_profile() {
        let backend = crate::test_utils::mock_backend();
        let config = AppConfig::default(); // no profiles

        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("renders");

        let result = render_short(
            &backend,
            &config,
            &input,
            &output_dir,
            "nonexistent",
            None,
            None,
            None,
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_render_short_with_event_metadata() {
        let backend = crate::test_utils::mock_backend();
        let config = {
            let mut c = AppConfig::default();
            let profile: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "tiktok",
                "width": 1080,
                "height": 1920
            }))
            .unwrap();
            c.render_profiles.insert("tiktok".to_string(), profile);
            c
        };
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("renders");

        let mut metadata = HashMap::new();
        metadata.insert("scorer".to_string(), "Player One".to_string());
        metadata.insert("team".to_string(), "Team A".to_string());

        let result = render_short(
            &backend,
            &config,
            &input,
            &output_dir,
            "tiktok",
            Some(&metadata),
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    // -----------------------------------------------------------------------
    // render_iteration — mock backend
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_iteration_single_item() {
        let backend = crate::test_utils::mock_backend();
        let config = {
            let mut c = AppConfig::default();
            let profile: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "tiktok",
                "width": 1080,
                "height": 1920
            }))
            .unwrap();
            c.render_profiles.insert("tiktok".to_string(), profile);
            c
        };
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("renders");

        let items = vec![IterationItem {
            profile_name: "tiktok".to_string(),
            overrides: None,
        }];

        let result = render_iteration(
            &backend,
            &config,
            &input,
            &output_dir,
            &items,
            false,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].format, "tiktok");
    }

    #[test]
    fn test_render_iteration_multiple_items() {
        let backend = crate::test_utils::mock_backend();
        let config = {
            let mut c = AppConfig::default();
            let p1: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "tiktok", "width": 1080, "height": 1920
            }))
            .unwrap();
            let p2: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "youtube", "width": 1920, "height": 1080
            }))
            .unwrap();
            c.render_profiles.insert("tiktok".to_string(), p1);
            c.render_profiles.insert("youtube".to_string(), p2);
            c
        };
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("renders");

        let items = vec![
            IterationItem {
                profile_name: "tiktok".to_string(),
                overrides: None,
            },
            IterationItem {
                profile_name: "youtube".to_string(),
                overrides: None,
            },
        ];

        let result = render_iteration(
            &backend,
            &config,
            &input,
            &output_dir,
            &items,
            false,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].format, "tiktok");
        assert_eq!(entries[1].format, "youtube");
    }

    #[test]
    fn test_render_iteration_empty_items_error() {
        let backend = crate::test_utils::mock_backend();
        let config = AppConfig::default();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("renders");

        let result = render_iteration(
            &backend,
            &config,
            &input,
            &output_dir,
            &[],
            false,
            None,
            None,
            None,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No iteration items provided");
    }

    #[test]
    fn test_render_iteration_concat_output() {
        let backend = crate::test_utils::mock_backend();
        let config = {
            let mut c = AppConfig::default();
            let p1: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "tiktok", "width": 1080, "height": 1920
            }))
            .unwrap();
            let p2: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "youtube", "width": 1920, "height": 1080
            }))
            .unwrap();
            c.render_profiles.insert("tiktok".to_string(), p1);
            c.render_profiles.insert("youtube".to_string(), p2);
            c
        };
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("renders");

        let items = vec![
            IterationItem {
                profile_name: "tiktok".to_string(),
                overrides: None,
            },
            IterationItem {
                profile_name: "youtube".to_string(),
                overrides: None,
            },
        ];

        let result = render_iteration(
            &backend,
            &config,
            &input,
            &output_dir,
            &items,
            true,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
        let entries = result.unwrap();
        // Concat produces a single entry
        assert_eq!(entries.len(), 1);
        assert!(entries[0].output.contains("iteration"));
        assert_eq!(entries[0].format, "tiktok+youtube");
    }

    // -----------------------------------------------------------------------
    // render_preview — mock backend
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_preview_default_fallback() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("previews");

        let result = render_preview(&backend, &input, &output_dir, None, None, None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.exists());
        assert!(output.to_str().unwrap().contains("preview"));
    }

    #[test]
    fn test_render_preview_with_profile() {
        let backend = crate::test_utils::mock_backend();
        let config = {
            let mut c = AppConfig::default();
            let profile: RenderProfile = serde_json::from_value(serde_json::json!({
                "name": "tiktok",
                "width": 1080,
                "height": 1920
            }))
            .unwrap();
            c.render_profiles.insert("tiktok".to_string(), profile);
            c
        };
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let output_dir = dir.path().join("previews");

        let result = render_preview(
            &backend,
            &input,
            &output_dir,
            Some(&config),
            Some("tiktok"),
            None,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.exists());
        assert!(output.to_str().unwrap().contains("tiktok_preview"));
    }

    // -----------------------------------------------------------------------
    // render_reel — mock backend
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_reel_two_files() {
        let backend = crate::test_utils::mock_backend();
        let config = AppConfig::default();
        let dir = tempfile::tempdir().unwrap();

        let short1 = dir.path().join("short1.mp4");
        let short2 = dir.path().join("short2.mp4");
        std::fs::write(&short1, "fake1").unwrap();
        std::fs::write(&short2, "fake2").unwrap();
        let output = dir.path().join("reel.mp4");

        let result = render_reel(&backend, &config, &[short1, short2], &output, None);
        assert!(result.is_ok());
        let rr = result.unwrap();
        assert!(rr.output.exists());
        assert!(rr.duration_secs > 0.0);
    }

    #[test]
    fn test_render_reel_empty_shorts_error() {
        let backend = crate::test_utils::mock_backend();
        let config = AppConfig::default();
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("reel.mp4");

        let result = render_reel(&backend, &config, &[], &output, None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No shorts to concatenate into a reel");
    }

    // -----------------------------------------------------------------------
    // list_render_profiles
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_render_profiles_with_profiles() {
        let mut config = AppConfig::default();
        let p1: RenderProfile = serde_json::from_value(serde_json::json!({
            "width": 1080, "height": 1920
        }))
        .unwrap();
        let p2: RenderProfile = serde_json::from_value(serde_json::json!({
            "name": "youtube", "width": 1920, "height": 1080
        }))
        .unwrap();
        config.render_profiles.insert("tiktok".to_string(), p1);
        config.render_profiles.insert("youtube".to_string(), p2);

        let profiles = list_render_profiles(&config);
        assert_eq!(profiles.len(), 2);

        // Profile without a name should get the key as name
        let tiktok = profiles.iter().find(|p| p.name == "tiktok").unwrap();
        assert_eq!(tiktok.width, Some(1080));

        // Profile with an existing name should keep it
        let youtube = profiles.iter().find(|p| p.name == "youtube").unwrap();
        assert_eq!(youtube.width, Some(1920));
    }

    #[test]
    fn test_list_render_profiles_empty_config() {
        let config = AppConfig::default();
        let profiles = list_render_profiles(&config);
        assert!(profiles.is_empty());
    }

    // -----------------------------------------------------------------------
    // render_profile_preview
    // -----------------------------------------------------------------------

    #[test]
    fn test_render_profile_preview_basic() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({"name": "test", "width": 1080, "height": 1920});

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.to_str().unwrap().contains("profile_preview"));
    }

    #[test]
    fn test_render_profile_preview_with_crop() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({
            "name": "crop-test",
            "width": 1080,
            "height": 1920,
            "crop_mode": "crop"
        });

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_profile_preview_with_pad() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({
            "name": "pad-test",
            "width": 1080,
            "height": 1920,
            "crop_mode": "pad",
            "pad_color": "#ff0000"
        });

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_profile_preview_with_speed() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({"name": "speed-test", "speed": 1.5});

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_profile_preview_with_lut() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({"name": "lut-test", "lut": "/path/to/color.cube"});

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_profile_preview_empty_profile_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({});

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_profile_preview_width_only() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({"width": 720});

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_profile_preview_height_only() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({"height": 480});

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_profile_preview_with_config_codec() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let mut config = AppConfig::default();
        config.video.codec = "libx265".to_string();

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({"name": "with-config"});

        let result =
            render_profile_preview(&backend, &input, &out_dir, Some(&config), &profile, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_profile_preview_empty_lut_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        let profile = serde_json::json!({"name": "empty-lut", "lut": ""});

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_profile_preview_speed_1_no_filter() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, "fake").unwrap();
        let out_dir = dir.path().join("renders");

        let backend = crate::test_utils::mock_backend();
        // Speed of 1.0 should NOT add speed filter
        let profile = serde_json::json!({"name": "normal-speed", "speed": 1.0});

        let result = render_profile_preview(&backend, &input, &out_dir, None, &profile, None);
        assert!(result.is_ok());
    }

    // ── prepare_preview_proxy ─────────────────────────────────────

    #[test]
    fn test_proxy_mp4_passthrough() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        std::fs::write(&input, b"fake_mp4").unwrap();

        let result = prepare_preview_proxy(&backend, &input, &dir.path().join("proxies"));
        assert_eq!(result.unwrap(), input);
    }

    #[test]
    fn test_proxy_mov_passthrough() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mov");
        std::fs::write(&input, b"fake_mov").unwrap();

        let result = prepare_preview_proxy(&backend, &input, &dir.path().join("proxies"));
        assert_eq!(result.unwrap(), input);
    }

    #[test]
    fn test_proxy_webm_passthrough() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.webm");
        std::fs::write(&input, b"fake_webm").unwrap();

        let result = prepare_preview_proxy(&backend, &input, &dir.path().join("proxies"));
        assert_eq!(result.unwrap(), input);
    }

    #[test]
    fn test_proxy_mkv_generates_mp4() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mkv");
        std::fs::write(&input, b"fake_mkv").unwrap();
        let proxy_dir = dir.path().join("proxies");

        let result = prepare_preview_proxy(&backend, &input, &proxy_dir).unwrap();

        assert!(result.to_string_lossy().ends_with(".mp4"));
        assert!(result.is_file());
        assert_ne!(result, input);
        // Should be in proxy dir
        assert!(result.starts_with(&proxy_dir));
    }

    #[test]
    fn test_proxy_avi_generates_mp4() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.avi");
        std::fs::write(&input, b"fake_avi").unwrap();

        let result = prepare_preview_proxy(&backend, &input, &dir.path().join("proxies")).unwrap();
        assert!(result.to_string_lossy().ends_with(".mp4"));
        assert!(result.is_file());
    }

    #[test]
    fn test_proxy_ts_generates_mp4() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.ts");
        std::fs::write(&input, b"fake_ts").unwrap();

        let result = prepare_preview_proxy(&backend, &input, &dir.path().join("proxies")).unwrap();
        assert!(result.to_string_lossy().ends_with(".mp4"));
    }

    #[test]
    fn test_proxy_cache_hit() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mkv");
        std::fs::write(&input, b"fake_mkv").unwrap();
        let proxy_dir = dir.path().join("proxies");

        // First call generates proxy
        let first = prepare_preview_proxy(&backend, &input, &proxy_dir).unwrap();
        let content_before = std::fs::read(&first).unwrap();

        // Second call returns cached proxy (file untouched)
        let second = prepare_preview_proxy(&backend, &input, &proxy_dir).unwrap();
        let content_after = std::fs::read(&second).unwrap();

        assert_eq!(first, second);
        assert_eq!(content_before, content_after);
    }

    #[test]
    fn test_proxy_creates_proxy_dir() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mkv");
        std::fs::write(&input, b"fake_mkv").unwrap();
        let proxy_dir = dir.path().join("deep").join("nested").join("proxies");

        assert!(!proxy_dir.exists());
        let result = prepare_preview_proxy(&backend, &input, &proxy_dir);
        assert!(result.is_ok());
        assert!(proxy_dir.exists());
    }

    #[test]
    fn test_proxy_deterministic_naming() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mkv");
        std::fs::write(&input, b"fake_mkv").unwrap();
        let proxy_dir = dir.path().join("proxies");

        let first = prepare_preview_proxy(&backend, &input, &proxy_dir).unwrap();
        let second = prepare_preview_proxy(&backend, &input, &proxy_dir).unwrap();
        assert_eq!(first, second);
        // Name includes stem
        assert!(
            first
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("clip")
        );
    }

    #[test]
    fn test_proxy_case_insensitive_extension() {
        let backend = crate::test_utils::mock_backend();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.MP4");
        std::fs::write(&input, b"fake_mp4").unwrap();

        let result = prepare_preview_proxy(&backend, &input, &dir.path().join("proxies")).unwrap();
        // .MP4 should be treated as web-playable
        assert_eq!(result, input);
    }

    // ── cleanup_proxy_cache ───────────────────────────────────────

    #[test]
    fn test_cleanup_proxy_cache_removes_old_files() {
        let dir = tempfile::tempdir().unwrap();
        let proxy_dir = dir.path().join("proxies");
        std::fs::create_dir_all(&proxy_dir).unwrap();

        let old_file = proxy_dir.join("old_clip.mp4");
        std::fs::write(&old_file, b"old").unwrap();
        // Set modification time to 30 days ago
        let thirty_days_ago =
            std::time::SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 60 * 60);
        filetime::set_file_mtime(
            &old_file,
            filetime::FileTime::from_system_time(thirty_days_ago),
        )
        .unwrap();

        let new_file = proxy_dir.join("new_clip.mp4");
        std::fs::write(&new_file, b"new").unwrap();

        cleanup_proxy_cache(&proxy_dir, std::time::Duration::from_secs(7 * 24 * 60 * 60));

        assert!(!old_file.exists());
        assert!(new_file.exists());
    }

    #[test]
    fn test_cleanup_proxy_cache_no_dir() {
        // Should not panic when directory doesn't exist
        cleanup_proxy_cache(
            Path::new("/nonexistent/proxy/dir"),
            std::time::Duration::from_secs(1),
        );
    }
}
