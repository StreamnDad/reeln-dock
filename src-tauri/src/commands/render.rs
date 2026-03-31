use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::orchestration::{progress::ProgressReporter, render_ops};
use crate::state::AppState;

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
) -> Result<serde_json::Value, String> {
    let backend = state.media_backend.clone();
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id.clone());

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
            None,
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
) -> Result<serde_json::Value, String> {
    let backend = state.media_backend.clone();
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id.clone());

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
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    let mut entries = result?;

    // Tag entries with event_id and save to game state
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
