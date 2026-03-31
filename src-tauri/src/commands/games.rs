use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use reeln_sport::default_event_type_entries;

use crate::models::GameSummary;
use crate::orchestration::{game_ops, progress::ProgressReporter};
use crate::state::AppState;

#[tauri::command]
pub fn update_game_event(
    game_dir: String,
    event_id: String,
    field: String,
    value: String,
) -> Result<reeln_state::GameState, String> {
    let path = Path::new(&game_dir);
    let mut state = reeln_state::load_game_state(path).map_err(|e| e.to_string())?;

    let event = state
        .events
        .iter_mut()
        .find(|e| e.id == event_id)
        .ok_or_else(|| format!("Event {} not found", event_id))?;

    match field.as_str() {
        "clip" => event.clip = value,
        "event_type" => event.event_type = value,
        "player" => event.player = value,
        other => {
            // Handle metadata fields (e.g. "scorer", "assist1", "assist2")
            if value.is_empty() {
                event.metadata.remove(other);
            } else {
                event.metadata.insert(
                    other.to_string(),
                    serde_json::Value::String(value),
                );
            }
        }
    }

    reeln_state::save_game_state(&state, path).map_err(|e| e.to_string())?;
    Ok(state)
}

#[tauri::command]
pub fn list_games(output_dir: String) -> Result<Vec<GameSummary>, String> {
    let base = Path::new(&output_dir);
    if !base.is_dir() {
        return Ok(Vec::new());
    }

    let mut games = Vec::new();
    let entries = std::fs::read_dir(base).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let game_json = path.join("game.json");
            if game_json.exists() {
                match reeln_state::load_game_state(&path) {
                    Ok(state) => games.push(GameSummary {
                        dir_path: path.display().to_string(),
                        state,
                    }),
                    Err(_) => continue,
                }
            }
        }
    }

    Ok(games)
}

#[tauri::command]
pub fn get_game_state(game_dir: String) -> Result<reeln_state::GameState, String> {
    let path = Path::new(&game_dir);
    reeln_state::load_game_state(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_game_tournament(game_dir: String, tournament: String) -> Result<reeln_state::GameState, String> {
    let path = Path::new(&game_dir);
    let mut state = reeln_state::load_game_state(path).map_err(|e| e.to_string())?;
    state.game_info.tournament = tournament;
    reeln_state::save_game_state(&state, path).map_err(|e| e.to_string())?;
    Ok(state)
}

#[tauri::command]
pub async fn init_game(
    state: State<'_, AppState>,
    sport: String,
    home_team: String,
    away_team: String,
    date: String,
    venue: Option<String>,
    game_time: Option<String>,
    level: Option<String>,
    tournament: Option<String>,
    period_length: Option<u32>,
) -> Result<GameSummary, String> {
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;
    let registry = state
        .sport_registry
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let result = tokio::task::spawn_blocking(move || {
        let params = game_ops::InitGameParams {
            sport,
            home_team,
            away_team,
            date,
            venue,
            game_time,
            level,
            tournament,
            period_length,
        };
        game_ops::init_game(&config, &registry, params)
    })
    .await
    .map_err(|e| e.to_string())?;

    let (game_dir, game_state) = result?;

    Ok(GameSummary {
        dir_path: game_dir.display().to_string(),
        state: game_state,
    })
}

#[tauri::command]
pub async fn process_segment(
    app: AppHandle,
    state: State<'_, AppState>,
    game_dir: String,
    segment_number: u32,
) -> Result<serde_json::Value, String> {
    let backend = state.media_backend.clone();
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id);
    let game_path = PathBuf::from(&game_dir);

    let result = tokio::task::spawn_blocking(move || {
        game_ops::process_segment(&backend, &config, &game_path, segment_number, Some(&reporter))
    })
    .await
    .map_err(|e| e.to_string())?;

    let game_state = result?;
    serde_json::to_value(&game_state).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn merge_highlights(
    app: AppHandle,
    state: State<'_, AppState>,
    game_dir: String,
) -> Result<serde_json::Value, String> {
    let backend = state.media_backend.clone();
    let config = state
        .config
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| "Config not loaded".to_string())?;

    let job_id = uuid::Uuid::new_v4().to_string();
    let reporter = ProgressReporter::new(app, job_id);
    let game_path = PathBuf::from(&game_dir);

    let result = tokio::task::spawn_blocking(move || {
        game_ops::merge_highlights(&backend, &config, &game_path, Some(&reporter))
    })
    .await
    .map_err(|e| e.to_string())?;

    let game_state = result?;
    serde_json::to_value(&game_state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn finish_game(game_dir: String) -> Result<serde_json::Value, String> {
    let game_state = game_ops::finish_game(Path::new(&game_dir))?;
    serde_json::to_value(&game_state).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn bulk_update_event_type(
    game_dir: String,
    event_ids: Vec<String>,
    event_type: String,
) -> Result<reeln_state::GameState, String> {
    let path = Path::new(&game_dir);
    let mut state = reeln_state::load_game_state(path).map_err(|e| e.to_string())?;

    for event in state.events.iter_mut() {
        if event_ids.contains(&event.id) {
            event.event_type = event_type.clone();
        }
    }

    reeln_state::save_game_state(&state, path).map_err(|e| e.to_string())?;
    Ok(state)
}

/// Normalized event type entry for frontend consumption.
#[derive(serde::Serialize)]
pub struct EventTypeResponse {
    pub name: String,
    pub team_specific: bool,
}

#[tauri::command]
pub fn get_event_types(
    state: State<'_, AppState>,
) -> Result<Vec<EventTypeResponse>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let config = config.as_ref().ok_or("Config not loaded")?;

    if !config.event_types.is_empty() {
        return Ok(config
            .event_types
            .iter()
            .map(|e| EventTypeResponse {
                name: e.name().to_string(),
                team_specific: e.team_specific(),
            })
            .collect());
    }

    // Fall back to sport defaults
    Ok(default_event_type_entries(&config.sport)
        .into_iter()
        .map(|(name, team_specific)| EventTypeResponse {
            name,
            team_specific,
        })
        .collect())
}

/// Remove render output files from disk and clear the renders array in game state.
/// Aligned with reeln-cli prune behavior for render artifacts.
#[tauri::command]
pub fn prune_renders(game_dir: String) -> Result<serde_json::Value, String> {
    let path = Path::new(&game_dir);
    let mut state = reeln_state::load_game_state(path).map_err(|e| e.to_string())?;

    let mut removed = 0u32;
    let mut bytes_freed = 0u64;

    for render in &state.renders {
        let output_path = Path::new(&render.output);
        if output_path.is_file() {
            if let Ok(meta) = std::fs::metadata(output_path) {
                bytes_freed += meta.len();
            }
            if std::fs::remove_file(output_path).is_ok() {
                removed += 1;
            }
        }
    }

    let total = state.renders.len() as u32;
    state.renders.clear();
    reeln_state::save_game_state(&state, path).map_err(|e| e.to_string())?;

    serde_json::to_value(&serde_json::json!({
        "state": state,
        "removed_files": removed,
        "cleared_entries": total,
        "bytes_freed": bytes_freed,
    }))
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn quick_tag_event(
    game_dir: String,
    event_id: String,
    event_type: String,
    team: Option<String>,
) -> Result<reeln_state::GameState, String> {
    let path = Path::new(&game_dir);
    let mut state = reeln_state::load_game_state(path).map_err(|e| e.to_string())?;

    let event = state
        .events
        .iter_mut()
        .find(|e| e.id == event_id)
        .ok_or_else(|| format!("Event {} not found", event_id))?;

    event.event_type = event_type;

    if let Some(ref team_val) = team {
        event
            .metadata
            .insert("team".to_string(), serde_json::Value::String(team_val.clone()));
    } else {
        event.metadata.remove("team");
    }

    reeln_state::save_game_state(&state, path).map_err(|e| e.to_string())?;
    Ok(state)
}
