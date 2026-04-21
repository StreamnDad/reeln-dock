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
    player_numbers: Option<String>,
    debug: Option<bool>,
    config_path: Option<String>,
    no_branding: Option<bool>,
    queue: Option<bool>,
) -> Result<serde_json::Value, String> {
    let debug_flag = debug.unwrap_or(false);
    let no_branding_flag = no_branding.unwrap_or(false);
    let queue_flag = queue.unwrap_or(false);

    // Try CLI first (full features), fall back to native backend
    let cli_path = {
        let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
        hook_executor::detect_reeln_cli(settings.reeln_cli_path.as_deref()).ok()
    };

    if let (Some(cli), Some(gdir)) = (&cli_path, &game_dir) {
        // Use explicit config path (plugin profile) if provided, otherwise fall back to DockSettings
        let config_path = config_path.or_else(|| {
            state.dock_settings.lock().ok()
                .and_then(|s| s.reeln_config_path.clone())
        });

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
            extra: o.extra.clone(),
        });

        let input = PathBuf::from(&input_clip);
        let game_path = PathBuf::from(gdir.as_str());
        let pname = profile_name.clone();
        let render_mode = mode.clone();
        let eid = event_id.clone();
        let sc = scorer.clone();
        let a1 = assist1.clone();
        let a2 = assist2.clone();
        let pn = player_numbers.clone();
        let cli_owned = cli.clone();
        let et = event_type;

        let result = tokio::task::spawn_blocking(move || {
            let profile_names: [&str; 1] = [pname.as_str()];
            let params = render_ops::CliRenderParams {
                cli_path: &cli_owned,
                config_path: config_path.as_deref(),
                input_clip: &input,
                game_dir: &game_path,
                profile_names: &profile_names,
                event_id: eid.as_deref(),
                mode: render_mode.as_deref(),
                overrides: ovr.as_ref(),
                scorer: sc.as_deref(),
                assist1: a1.as_deref(),
                assist2: a2.as_deref(),
                player_numbers: pn.as_deref(),
                iterate: false,
                event_type: et.as_deref(),
                debug: debug_flag,
                no_branding: no_branding_flag,
                output_path: None,
                queue: queue_flag,
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
        extra: o.extra,
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
        reeln_state::add_render(&mut game_state, entry.clone());
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
    player_numbers: Option<String>,
    debug: Option<bool>,
    config_path: Option<String>,
    no_branding: Option<bool>,
    queue: Option<bool>,
) -> Result<serde_json::Value, String> {
    let debug_flag = debug.unwrap_or(false);
    let no_branding_flag = no_branding.unwrap_or(false);
    let queue_flag = queue.unwrap_or(false);

    // Prefer the CLI path: it accepts multiple --render-profile flags and
    // does iteration + concat + queueing in a single call. The dock just
    // passes through the exact profile list the user picked, so the result
    // is one rendered file and (with --queue) one queue entry — no stale
    // per-profile artifacts, no cross-process game.json surgery.
    //
    // We deliberately do NOT use --iterate, which would discard the user's
    // explicit selections and re-resolve profiles from event-type config.
    //
    // NOTE: per-profile `overrides` inside each IterationItem are not
    // supported by this path — the CLI accepts overrides at the top level
    // only. The dock UI never populates per-profile overrides anyway, so we
    // take the overrides from the first item and rely on the frontend
    // merging top-level stage overrides into every item identically.
    let cli_path = {
        let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
        hook_executor::detect_reeln_cli(settings.reeln_cli_path.as_deref()).ok()
    };

    if let (Some(cli), Some(gdir)) = (&cli_path, &game_dir) {
        if items.is_empty() {
            return Err("render_iteration called with no profiles".to_string());
        }

        // Use explicit config path (plugin profile) if provided, otherwise fall back to DockSettings
        let config_path = config_path.or_else(|| {
            state.dock_settings.lock().ok()
                .and_then(|s| s.reeln_config_path.clone())
        });

        let event_type = event_id.as_deref().and_then(|eid| {
            let game_path = Path::new(gdir.as_str());
            reeln_state::load_game_state(game_path).ok().and_then(|s| {
                s.events.iter().find(|e| e.id == eid).map(|e| e.event_type.clone())
            })
        });

        let _ = concat_output; // CLI always concatenates when given ≥2 profiles

        let input = PathBuf::from(&input_clip);
        let game_path = PathBuf::from(gdir.as_str());
        let render_mode = mode.clone();
        let eid = event_id.clone();
        let sc = scorer.clone();
        let a1 = assist1.clone();
        let a2 = assist2.clone();
        let pn = player_numbers.clone();
        let cli_owned = cli.clone();
        let et = event_type;
        let items_owned = items.clone();
        let cfg_path = config_path.clone();

        let result = tokio::task::spawn_blocking(move || {
            // Flatten profile names into owned strings so the slice we pass
            // to render_via_cli lives as long as the call.
            let owned_names: Vec<String> = items_owned
                .iter()
                .map(|i| i.profile_name.clone())
                .collect();
            let name_refs: Vec<&str> = owned_names.iter().map(|s| s.as_str()).collect();

            // Take overrides from the first item; all items share them in
            // practice because the frontend merges top-level overrides
            // uniformly.
            let ovr = items_owned
                .first()
                .and_then(|item| item.overrides.as_ref())
                .map(|o| render_ops::RenderOverrides {
                    crop_mode: o.crop_mode.clone(),
                    scale: o.scale,
                    speed: o.speed,
                    smart: o.smart,
                    anchor_x: o.anchor_x,
                    anchor_y: o.anchor_y,
                    pad_color: o.pad_color.clone(),
                    zoom_frames: o.zoom_frames,
                    extra: o.extra.clone(),
                });

            let params = render_ops::CliRenderParams {
                cli_path: &cli_owned,
                config_path: cfg_path.as_deref(),
                input_clip: &input,
                game_dir: &game_path,
                profile_names: &name_refs,
                event_id: eid.as_deref(),
                mode: render_mode.as_deref(),
                overrides: ovr.as_ref(),
                scorer: sc.as_deref(),
                assist1: a1.as_deref(),
                assist2: a2.as_deref(),
                player_numbers: pn.as_deref(),
                iterate: false,
                event_type: et.as_deref(),
                debug: debug_flag,
                no_branding: no_branding_flag,
                output_path: None,
                queue: queue_flag,
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
                extra: o.extra,
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
        for entry in entries.clone() {
            reeln_state::add_render(&mut game_state, entry);
        }
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

/// Render a preview using an inline (unsaved) profile object.
/// Used by the profile editor so users can preview changes before saving.
#[tauri::command]
pub async fn render_profile_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    input_clip: String,
    output_dir: String,
    profile: serde_json::Value,
) -> Result<String, String> {
    let backend = state.media_backend.clone();
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let input = PathBuf::from(&input_clip);
    let out_dir = PathBuf::from(&output_dir);

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id);

    let result = tokio::task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            render_ops::render_profile_preview(
                &backend,
                &input,
                &out_dir,
                config.as_ref(),
                &profile,
                Some(&reporter),
            )
        }))
        .unwrap_or_else(|_| Err("Preview render crashed — try different profile settings".to_string()))
    })
    .await
    .map_err(|e| e.to_string())?;

    result.map(|p| p.display().to_string())
}

/// Suggest a clip for preview by scanning recent games.
#[tauri::command]
pub fn suggest_preview_clip(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let config = match config {
        Some(c) => c,
        None => return Ok(None),
    };

    let output_dir = match &config.paths.output_dir {
        Some(p) if p.is_dir() => p,
        _ => return Ok(None),
    };

    // Scan for game directories, sorted by modification time (newest first)
    let mut game_dirs: Vec<_> = std::fs::read_dir(output_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((entry.path(), modified))
        })
        .collect();

    game_dirs.sort_by(|a, b| b.1.cmp(&a.1));

    // Find the first game with an event that has a clip
    for (game_path, _) in game_dirs.iter().take(10) {
        if let Ok(game_state) = reeln_state::load_game_state(game_path) {
            for event in &game_state.events {
                let clip_path = game_path.join(&event.clip);
                if clip_path.is_file() {
                    return Ok(Some(clip_path.display().to_string()));
                }
            }
        }
    }

    Ok(None)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ── build_event_metadata ─────────────────────────────────────────

    #[test]
    fn build_event_metadata_basic_game_info() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        crate::test_utils::create_test_game(&game_dir);

        let meta =
            build_event_metadata(game_dir.to_str().unwrap(), None, None, None, None).unwrap();

        assert_eq!(meta.get("home_team").unwrap(), "Team A");
        assert_eq!(meta.get("away_team").unwrap(), "Team B");
        assert_eq!(meta.get("date").unwrap(), "2026-04-03");
        assert_eq!(meta.get("sport").unwrap(), "soccer");
        assert_eq!(meta.get("tournament").unwrap(), "Test League");
        // level is empty string — should still be in map (it's always inserted)
        assert!(meta.contains_key("level"));
        // venue is empty — should NOT be in map
        assert!(!meta.contains_key("venue"));
    }

    #[test]
    fn build_event_metadata_with_venue() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game_venue");
        let mut state = crate::test_utils::create_test_game(&game_dir);
        state.game_info.venue = "Stadium".to_string();
        reeln_state::save_game_state(&state, &game_dir).unwrap();

        let meta =
            build_event_metadata(game_dir.to_str().unwrap(), None, None, None, None).unwrap();

        assert_eq!(meta.get("venue").unwrap(), "Stadium");
    }

    #[test]
    fn build_event_metadata_with_matching_event() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game_event");
        let mut state = crate::test_utils::create_test_game(&game_dir);
        state.events.push(reeln_state::GameEvent {
            id: "evt1".to_string(),
            clip: "clip.mp4".to_string(),
            segment_number: 2,
            event_type: "goal".to_string(),
            player: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        });
        reeln_state::save_game_state(&state, &game_dir).unwrap();

        let meta = build_event_metadata(
            game_dir.to_str().unwrap(),
            Some("evt1"),
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(meta.get("event_type").unwrap(), "goal");
        assert_eq!(meta.get("segment_number").unwrap(), "2");
    }

    #[test]
    fn build_event_metadata_event_metadata_string_values() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game_evt_meta");
        let mut state = crate::test_utils::create_test_game(&game_dir);
        let mut evt_meta = HashMap::new();
        evt_meta.insert(
            "team".to_string(),
            serde_json::Value::String("home".to_string()),
        );
        // Non-string values should be skipped
        evt_meta.insert("score".to_string(), serde_json::Value::Number(3.into()));
        state.events.push(reeln_state::GameEvent {
            id: "evt2".to_string(),
            clip: "clip2.mp4".to_string(),
            segment_number: 1,
            event_type: "assist".to_string(),
            player: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: evt_meta,
        });
        reeln_state::save_game_state(&state, &game_dir).unwrap();

        let meta = build_event_metadata(
            game_dir.to_str().unwrap(),
            Some("evt2"),
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(meta.get("team").unwrap(), "home");
        // Non-string metadata value should not be present
        assert!(!meta.contains_key("score"));
    }

    #[test]
    fn build_event_metadata_scorer_assist_override() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game_players");
        crate::test_utils::create_test_game(&game_dir);

        let meta = build_event_metadata(
            game_dir.to_str().unwrap(),
            None,
            Some("PlayerA"),
            Some("PlayerB"),
            Some("PlayerC"),
        )
        .unwrap();

        assert_eq!(meta.get("player").unwrap(), "PlayerA");
        assert_eq!(meta.get("assist1").unwrap(), "PlayerB");
        assert_eq!(meta.get("assist2").unwrap(), "PlayerC");
    }

    #[test]
    fn build_event_metadata_empty_scorer_assist_not_included() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game_empty_players");
        crate::test_utils::create_test_game(&game_dir);

        let meta = build_event_metadata(
            game_dir.to_str().unwrap(),
            None,
            Some(""),
            Some(""),
            Some(""),
        )
        .unwrap();

        assert!(!meta.contains_key("player"));
        assert!(!meta.contains_key("assist1"));
        assert!(!meta.contains_key("assist2"));
    }

    #[test]
    fn build_event_metadata_event_not_found_still_returns_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game_no_evt");
        crate::test_utils::create_test_game(&game_dir);

        let meta = build_event_metadata(
            game_dir.to_str().unwrap(),
            Some("nonexistent"),
            None,
            None,
            None,
        )
        .unwrap();

        // Basic info is present even though event was not found
        assert_eq!(meta.get("home_team").unwrap(), "Team A");
        assert!(!meta.contains_key("event_type"));
        assert!(!meta.contains_key("segment_number"));
    }

    #[test]
    fn build_event_metadata_invalid_game_dir_returns_none() {
        let result = build_event_metadata("/nonexistent/path", None, None, None, None);
        assert!(result.is_none());
    }

    // ── delete_preview ───────────────────────────────────────────────

    #[test]
    fn delete_preview_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("preview.mp4");
        std::fs::write(&file_path, b"preview data").unwrap();
        assert!(file_path.exists());

        let result = delete_preview(file_path.to_str().unwrap().to_string());
        assert!(result.is_ok());
        assert!(!file_path.exists());
    }

    #[test]
    fn delete_preview_nonexistent_file_ok() {
        let result = delete_preview("/nonexistent/preview.mp4".to_string());
        assert!(result.is_ok());
    }

    // ── RenderOverrides serde ────────────────────────────────────────

    #[test]
    fn render_overrides_default_all_none() {
        let ovr = RenderOverrides::default();
        assert!(ovr.crop_mode.is_none());
        assert!(ovr.scale.is_none());
        assert!(ovr.speed.is_none());
        assert!(ovr.smart.is_none());
        assert!(ovr.anchor_x.is_none());
        assert!(ovr.anchor_y.is_none());
        assert!(ovr.pad_color.is_none());
        assert!(ovr.zoom_frames.is_none());
        assert!(ovr.extra.is_empty());
    }

    #[test]
    fn render_overrides_deserialize_all_fields() {
        let json = serde_json::json!({
            "crop_mode": "center",
            "scale": 0.5,
            "speed": 1.5,
            "smart": true,
            "anchor_x": 0.3,
            "anchor_y": 0.7,
            "pad_color": "#000000",
            "zoom_frames": 10,
            "custom_field": "custom_value"
        });

        let ovr: RenderOverrides = serde_json::from_value(json).unwrap();
        assert_eq!(ovr.crop_mode.as_deref(), Some("center"));
        assert_eq!(ovr.scale, Some(0.5));
        assert_eq!(ovr.speed, Some(1.5));
        assert_eq!(ovr.smart, Some(true));
        assert_eq!(ovr.anchor_x, Some(0.3));
        assert_eq!(ovr.anchor_y, Some(0.7));
        assert_eq!(ovr.pad_color.as_deref(), Some("#000000"));
        assert_eq!(ovr.zoom_frames, Some(10));
        assert_eq!(
            ovr.extra.get("custom_field").unwrap(),
            &serde_json::Value::String("custom_value".to_string())
        );
    }

    #[test]
    fn render_overrides_roundtrip() {
        let ovr = RenderOverrides {
            crop_mode: Some("letterbox".to_string()),
            scale: Some(1.0),
            speed: Some(2.0),
            smart: Some(false),
            anchor_x: Some(0.5),
            anchor_y: Some(0.5),
            pad_color: Some("#ffffff".to_string()),
            zoom_frames: Some(5),
            extra: {
                let mut m = HashMap::new();
                m.insert(
                    "plugin_key".to_string(),
                    serde_json::Value::Bool(true),
                );
                m
            },
        };

        let serialized = serde_json::to_string(&ovr).unwrap();
        let deserialized: RenderOverrides = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.crop_mode, ovr.crop_mode);
        assert_eq!(deserialized.scale, ovr.scale);
        assert_eq!(deserialized.speed, ovr.speed);
        assert_eq!(deserialized.smart, ovr.smart);
        assert_eq!(deserialized.anchor_x, ovr.anchor_x);
        assert_eq!(deserialized.anchor_y, ovr.anchor_y);
        assert_eq!(deserialized.pad_color, ovr.pad_color);
        assert_eq!(deserialized.zoom_frames, ovr.zoom_frames);
        assert_eq!(deserialized.extra, ovr.extra);
    }

    // ── IterationItem serde ──────────────────────────────────────────

    #[test]
    fn iteration_item_with_overrides() {
        let json = serde_json::json!({
            "profile_name": "default",
            "overrides": {
                "speed": 1.5,
                "smart": true
            }
        });

        let item: IterationItem = serde_json::from_value(json).unwrap();
        assert_eq!(item.profile_name, "default");
        let ovr = item.overrides.unwrap();
        assert_eq!(ovr.speed, Some(1.5));
        assert_eq!(ovr.smart, Some(true));
    }

    #[test]
    fn iteration_item_without_overrides() {
        let json = serde_json::json!({
            "profile_name": "fast",
            "overrides": null
        });

        let item: IterationItem = serde_json::from_value(json).unwrap();
        assert_eq!(item.profile_name, "fast");
        assert!(item.overrides.is_none());
    }
}
