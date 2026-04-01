use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::orchestration::{hook_executor, progress::ProgressReporter, render_ops};
use crate::state::AppState;

/// Build event metadata HashMap from game state + player params for overlay rendering.
fn build_event_metadata(
    game_dir: &str,
    event_id: Option<&str>,
    scorer: Option<&str>,
    assist1: Option<&str>,
    assist2: Option<&str>,
) -> Option<HashMap<String, String>> {
    let game_path = Path::new(game_dir);
    let state = reeln_state::load_game_state(game_path).ok()?;
    let info = &state.game_info;

    let mut meta = HashMap::new();
    meta.insert("home_team".to_string(), info.home_team.clone());
    meta.insert("away_team".to_string(), info.away_team.clone());
    meta.insert("date".to_string(), info.date.clone());
    meta.insert("sport".to_string(), info.sport.clone());
    meta.insert("tournament".to_string(), info.tournament.clone());
    meta.insert("level".to_string(), info.level.clone());
    if !info.venue.is_empty() {
        meta.insert("venue".to_string(), info.venue.clone());
    }

    // Event-specific metadata
    if let Some(eid) = event_id {
        if let Some(event) = state.events.iter().find(|e| e.id == eid) {
            meta.insert("event_type".to_string(), event.event_type.clone());
            meta.insert(
                "segment_number".to_string(),
                event.segment_number.to_string(),
            );
            // Include event metadata (e.g. team)
            for (k, v) in &event.metadata {
                if let Some(s) = v.as_str() {
                    meta.insert(k.clone(), s.to_string());
                }
            }
        }
    }

    // Player data from explicit params (override event metadata)
    if let Some(s) = scorer {
        if !s.is_empty() {
            meta.insert("player".to_string(), s.to_string());
        }
    }
    if let Some(a) = assist1 {
        if !a.is_empty() {
            meta.insert("assist1".to_string(), a.to_string());
        }
    }
    if let Some(a) = assist2 {
        if !a.is_empty() {
            meta.insert("assist2".to_string(), a.to_string());
        }
    }

    Some(meta)
}

/// Optional overrides for render profile parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderOverrides {
    pub crop_mode: Option<String>,
    pub scale: Option<f64>,
    pub speed: Option<f64>,
    pub smart: Option<bool>,
    pub anchor_x: Option<f64>,
    pub anchor_y: Option<f64>,
    pub pad_color: Option<String>,
    pub zoom_frames: Option<u32>,
    /// Additional plugin-contributed fields (passed through as-is).
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// A single item in a render iteration queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationItem {
    pub profile_name: String,
    pub overrides: Option<RenderOverrides>,
}

#[tauri::command]
pub async fn render_short(
    app: AppHandle,
    state: State<'_, AppState>,
    input_clip: String,
    output_dir: String,
    profile_name: String,
    event_id: Option<String>,
    game_dir: Option<String>,
    overrides: Option<RenderOverrides>,
    mode: Option<String>,
    scorer: Option<String>,
    assist1: Option<String>,
    assist2: Option<String>,
) -> Result<serde_json::Value, String> {
    // Try CLI first (full features), fall back to native backend
    let cli_path = {
        let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
        hook_executor::detect_reeln_cli(settings.reeln_cli_path.as_deref()).ok()
    };

    if let (Some(cli), Some(gdir)) = (&cli_path, &game_dir) {
        let config_path = state.dock_settings.lock().map_err(|e| e.to_string())?
            .reeln_config_path.clone();

        // Build event_type from game state for scoring team resolution
        let event_type = event_id.as_deref().and_then(|eid| {
            let game_path = Path::new(gdir.as_str());
            reeln_state::load_game_state(game_path).ok().and_then(|s| {
                s.events.iter().find(|e| e.id == eid).map(|e| e.event_type.clone())
            })
        });

        let ovr = overrides.as_ref().map(|o| render_ops::RenderOverrides {
            crop_mode: o.crop_mode.clone(),
            scale: o.scale,
            speed: o.speed,
            smart: o.smart,
            anchor_x: o.anchor_x,
            anchor_y: o.anchor_y,
            pad_color: o.pad_color.clone(),
            zoom_frames: o.zoom_frames,
        });

        let input = PathBuf::from(&input_clip);
        let game_path = PathBuf::from(gdir.as_str());
        let pname = profile_name.clone();
        let render_mode = mode.clone();
        let eid = event_id.clone();
        let sc = scorer.clone();
        let a1 = assist1.clone();
        let a2 = assist2.clone();
        let cli_owned = cli.clone();
        let et = event_type;

        let result = tokio::task::spawn_blocking(move || {
            let params = render_ops::CliRenderParams {
                cli_path: &cli_owned,
                config_path: config_path.as_deref(),
                input_clip: &input,
                game_dir: &game_path,
                profile_name: &pname,
                event_id: eid.as_deref(),
                mode: render_mode.as_deref(),
                overrides: ovr.as_ref(),
                scorer: sc.as_deref(),
                assist1: a1.as_deref(),
                assist2: a2.as_deref(),
                iterate: false,
                event_type: et.as_deref(),
            };
            render_ops::render_via_cli(&params)
        })
        .await
        .map_err(|e| e.to_string())?;

        let entries = result?;
        // CLI already saved to game.json — return the new entries
        if entries.len() == 1 {
            return serde_json::to_value(&entries[0]).map_err(|e| e.to_string());
        }
        return serde_json::to_value(&entries).map_err(|e| e.to_string());
    }

    // ── Native backend fallback (no CLI) ──────────────────────────────
    let backend = state.media_backend.clone();
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id.clone());

    let event_meta = game_dir.as_deref().and_then(|gdir| {
        build_event_metadata(
            gdir,
            event_id.as_deref(),
            scorer.as_deref(),
            assist1.as_deref(),
            assist2.as_deref(),
        )
    });

    let input = PathBuf::from(&input_clip);
    let out_dir = PathBuf::from(&output_dir);
    let profile = profile_name.clone();
    let eid = event_id.clone();
    let gdir = game_dir.clone();
    let render_mode = mode;
    let ovr = overrides.map(|o| render_ops::RenderOverrides {
        crop_mode: o.crop_mode,
        scale: o.scale,
        speed: o.speed,
        smart: o.smart,
        anchor_x: o.anchor_x,
        anchor_y: o.anchor_y,
        pad_color: o.pad_color,
        zoom_frames: o.zoom_frames,
    });

    let result = tokio::task::spawn_blocking(move || {
        render_ops::render_short(
            &backend,
            &config,
            &input,
            &out_dir,
            &profile,
            event_meta.as_ref(),
            ovr.as_ref(),
            Some(&reporter),
            render_mode.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    let mut entry = result?;

    if let Some(ref eid) = eid {
        entry.event_id = eid.clone();
    }

    if let Some(ref gdir) = gdir {
        let game_path = Path::new(gdir);
        let mut game_state =
            reeln_state::load_game_state(game_path).map_err(|e| e.to_string())?;
        game_state.renders.push(entry.clone());
        reeln_state::save_game_state(&game_state, game_path).map_err(|e| e.to_string())?;
    }

    serde_json::to_value(&entry).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn render_iteration(
    app: AppHandle,
    state: State<'_, AppState>,
    input_clip: String,
    output_dir: String,
    items: Vec<IterationItem>,
    event_id: Option<String>,
    game_dir: Option<String>,
    concat_output: bool,
    mode: Option<String>,
    scorer: Option<String>,
    assist1: Option<String>,
    assist2: Option<String>,
) -> Result<serde_json::Value, String> {
    // Try CLI first — with --iterate, CLI handles multi-profile + concatenation
    let cli_path = {
        let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
        hook_executor::detect_reeln_cli(settings.reeln_cli_path.as_deref()).ok()
    };

    if let (Some(cli), Some(gdir)) = (&cli_path, &game_dir) {
        let config_path = state.dock_settings.lock().map_err(|e| e.to_string())?
            .reeln_config_path.clone();

        let event_type = event_id.as_deref().and_then(|eid| {
            let game_path = Path::new(gdir.as_str());
            reeln_state::load_game_state(game_path).ok().and_then(|s| {
                s.events.iter().find(|e| e.id == eid).map(|e| e.event_type.clone())
            })
        });

        // Use the first profile's name for the CLI --render-profile flag
        // (with --iterate, the CLI uses event type → profile mappings from config)
        let first_profile = items.first().map(|i| i.profile_name.clone()).unwrap_or_default();

        // Merge global overrides (if any)
        let ovr = items.first().and_then(|i| i.overrides.as_ref()).map(|o| render_ops::RenderOverrides {
            crop_mode: o.crop_mode.clone(),
            scale: o.scale,
            speed: o.speed,
            smart: o.smart,
            anchor_x: o.anchor_x,
            anchor_y: o.anchor_y,
            pad_color: o.pad_color.clone(),
            zoom_frames: o.zoom_frames,
        });

        let input = PathBuf::from(&input_clip);
        let game_path = PathBuf::from(gdir.as_str());
        let render_mode = mode.clone();
        let eid = event_id.clone();
        let sc = scorer.clone();
        let a1 = assist1.clone();
        let a2 = assist2.clone();
        let cli_owned = cli.clone();
        let et = event_type;

        let result = tokio::task::spawn_blocking(move || {
            let params = render_ops::CliRenderParams {
                cli_path: &cli_owned,
                config_path: config_path.as_deref(),
                input_clip: &input,
                game_dir: &game_path,
                profile_name: &first_profile,
                event_id: eid.as_deref(),
                mode: render_mode.as_deref(),
                overrides: ovr.as_ref(),
                scorer: sc.as_deref(),
                assist1: a1.as_deref(),
                assist2: a2.as_deref(),
                iterate: true,
                event_type: et.as_deref(),
            };
            render_ops::render_via_cli(&params)
        })
        .await
        .map_err(|e| e.to_string())?;

        let entries = result?;
        return serde_json::to_value(&entries).map_err(|e| e.to_string());
    }

    // ── Native backend fallback ───────────────────────────────────────
    let backend = state.media_backend.clone();
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id.clone());

    let event_meta = game_dir.as_deref().and_then(|gdir| {
        build_event_metadata(
            gdir,
            event_id.as_deref(),
            scorer.as_deref(),
            assist1.as_deref(),
            assist2.as_deref(),
        )
    });

    let input = PathBuf::from(&input_clip);
    let out_dir = PathBuf::from(&output_dir);
    let eid = event_id.clone();
    let gdir = game_dir.clone();
    let render_mode = mode;

    let iter_items: Vec<render_ops::IterationItem> = items
        .into_iter()
        .map(|item| render_ops::IterationItem {
            profile_name: item.profile_name,
            overrides: item.overrides.map(|o| render_ops::RenderOverrides {
                crop_mode: o.crop_mode,
                scale: o.scale,
                speed: o.speed,
                smart: o.smart,
                anchor_x: o.anchor_x,
                anchor_y: o.anchor_y,
                pad_color: o.pad_color,
                zoom_frames: o.zoom_frames,
            }),
        })
        .collect();

    let result = tokio::task::spawn_blocking(move || {
        render_ops::render_iteration(
            &backend,
            &config,
            &input,
            &out_dir,
            &iter_items,
            concat_output,
            Some(&reporter),
            render_mode.as_deref(),
            event_meta.as_ref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    let mut entries = result?;

    if let Some(ref eid) = eid {
        for entry in &mut entries {
            entry.event_id = eid.clone();
        }
    }

    if let Some(ref gdir) = gdir {
        let game_path = Path::new(gdir);
        let mut game_state =
            reeln_state::load_game_state(game_path).map_err(|e| e.to_string())?;
        game_state.renders.extend(entries.clone());
        reeln_state::save_game_state(&game_state, game_path).map_err(|e| e.to_string())?;
    }

    serde_json::to_value(&entries).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_iteration_profiles(
    state: State<'_, AppState>,
    event_type: String,
) -> Result<Vec<String>, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;

    Ok(config.iterations.profiles_for_event(&event_type))
}

#[tauri::command]
pub async fn render_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    input_clip: String,
    output_dir: String,
    profile_name: Option<String>,
) -> Result<String, String> {
    let backend = state.media_backend.clone();
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let input = PathBuf::from(&input_clip);
    let out_dir = PathBuf::from(&output_dir);
    let pname = profile_name;

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id);

    let result = tokio::task::spawn_blocking(move || {
        render_ops::render_preview(
            &backend,
            &input,
            &out_dir,
            config.as_ref(),
            pname.as_deref(),
            Some(&reporter),
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    result.map(|p| p.display().to_string())
}

#[tauri::command]
pub fn delete_preview(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if p.exists() {
        std::fs::remove_file(p).map_err(|e| e.to_string())
    } else {
        Ok(())
    }
}

#[tauri::command]
pub async fn render_reel(
    app: AppHandle,
    state: State<'_, AppState>,
    shorts: Vec<String>,
    output: String,
) -> Result<serde_json::Value, String> {
    let backend = state.media_backend.clone();
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;

    let short_paths: Vec<PathBuf> = shorts.iter().map(PathBuf::from).collect();
    let output_path = PathBuf::from(&output);

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id);

    let result = tokio::task::spawn_blocking(move || {
        render_ops::render_reel(&backend, &config, &short_paths, &output_path, Some(&reporter))
    })
    .await
    .map_err(|e| e.to_string())?;

    let render_result = result?;
    serde_json::to_value(&serde_json::json!({
        "output": render_result.output.display().to_string(),
        "duration_secs": render_result.duration_secs,
    }))
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_render_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;

    let profiles = render_ops::list_render_profiles(&config);
    profiles
        .iter()
        .map(|p| serde_json::to_value(p).map_err(|e| e.to_string()))
        .collect()
}
