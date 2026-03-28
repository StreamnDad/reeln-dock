use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};

use crate::orchestration::{progress::ProgressReporter, render_ops};
use crate::state::AppState;

#[tauri::command]
pub async fn render_short(
    app: AppHandle,
    state: State<'_, AppState>,
    input_clip: String,
    output_dir: String,
    profile_name: String,
    event_id: Option<String>,
    game_dir: Option<String>,
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

    let result = tokio::task::spawn_blocking(move || {
        render_ops::render_short(
            &backend,
            &config,
            &input,
            &out_dir,
            &profile,
            None,
            Some(&reporter),
        )
    })
    .await
    .map_err(|e| e.to_string())?;

    let mut entry = result?;

    // If event_id was provided, record the render in game state
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
pub async fn render_preview(
    app: AppHandle,
    state: State<'_, AppState>,
    input_clip: String,
    output_dir: String,
) -> Result<String, String> {
    let backend = state.media_backend.clone();
    let input = PathBuf::from(&input_clip);
    let out_dir = PathBuf::from(&output_dir);

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id);

    let result = tokio::task::spawn_blocking(move || {
        render_ops::render_preview(&backend, &input, &out_dir, Some(&reporter))
    })
    .await
    .map_err(|e| e.to_string())?;

    result.map(|p| p.display().to_string())
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
