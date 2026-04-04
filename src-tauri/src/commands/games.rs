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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper: create a test game and add events to it.
    fn setup_game_with_events(game_dir: &Path, events: Vec<reeln_state::GameEvent>) {
        let mut state = crate::test_utils::create_test_game(game_dir);
        state.events = events;
        reeln_state::save_game_state(&state, game_dir).unwrap();
    }

    fn make_event(id: &str, clip: &str, event_type: &str) -> reeln_state::GameEvent {
        reeln_state::GameEvent {
            id: id.to_string(),
            clip: clip.to_string(),
            segment_number: 1,
            event_type: event_type.to_string(),
            player: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        }
    }

    // ── update_game_event ──────────────────────────────────────────────

    #[test]
    fn update_game_event_sets_clip() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        setup_game_with_events(&game_dir, vec![make_event("evt1", "old.mp4", "goal")]);

        let result = update_game_event(
            game_dir.display().to_string(),
            "evt1".into(),
            "clip".into(),
            "new.mp4".into(),
        )
        .unwrap();

        assert_eq!(result.events[0].clip, "new.mp4");

        // Verify persisted
        let reloaded = reeln_state::load_game_state(&game_dir).unwrap();
        assert_eq!(reloaded.events[0].clip, "new.mp4");
    }

    #[test]
    fn update_game_event_sets_event_type() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        setup_game_with_events(&game_dir, vec![make_event("evt1", "clip.mp4", "goal")]);

        let result = update_game_event(
            game_dir.display().to_string(),
            "evt1".into(),
            "event_type".into(),
            "assist".into(),
        )
        .unwrap();

        assert_eq!(result.events[0].event_type, "assist");
    }

    #[test]
    fn update_game_event_sets_player() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        setup_game_with_events(&game_dir, vec![make_event("evt1", "clip.mp4", "goal")]);

        let result = update_game_event(
            game_dir.display().to_string(),
            "evt1".into(),
            "player".into(),
            "Player 7".into(),
        )
        .unwrap();

        assert_eq!(result.events[0].player, "Player 7");
    }

    #[test]
    fn update_game_event_sets_metadata_field() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        setup_game_with_events(&game_dir, vec![make_event("evt1", "clip.mp4", "goal")]);

        let result = update_game_event(
            game_dir.display().to_string(),
            "evt1".into(),
            "scorer".into(),
            "Player 9".into(),
        )
        .unwrap();

        assert_eq!(
            result.events[0].metadata.get("scorer"),
            Some(&serde_json::Value::String("Player 9".to_string()))
        );
    }

    #[test]
    fn update_game_event_removes_metadata_when_value_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        let mut evt = make_event("evt1", "clip.mp4", "goal");
        evt.metadata.insert(
            "scorer".to_string(),
            serde_json::Value::String("Player 9".to_string()),
        );
        setup_game_with_events(&game_dir, vec![evt]);

        let result = update_game_event(
            game_dir.display().to_string(),
            "evt1".into(),
            "scorer".into(),
            "".into(),
        )
        .unwrap();

        assert!(!result.events[0].metadata.contains_key("scorer"));
    }

    #[test]
    fn update_game_event_returns_error_for_missing_event() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        setup_game_with_events(&game_dir, vec![make_event("evt1", "clip.mp4", "goal")]);

        let err = update_game_event(
            game_dir.display().to_string(),
            "nonexistent".into(),
            "clip".into(),
            "x.mp4".into(),
        )
        .unwrap_err();

        assert!(err.contains("nonexistent"));
        assert!(err.contains("not found"));
    }

    // ── list_games ─────────────────────────────────────────────────────

    #[test]
    fn list_games_returns_empty_for_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = list_games(tmp.path().display().to_string()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_games_finds_game_with_game_json() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        crate::test_utils::create_test_game(&game_dir);

        let result = list_games(tmp.path().display().to_string()).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].dir_path.contains("game1"));
        assert_eq!(result[0].state.game_info.home_team, "Team A");
    }

    #[test]
    fn list_games_skips_dir_without_game_json() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("not_a_game");
        std::fs::create_dir_all(&subdir).unwrap();

        let result = list_games(tmp.path().display().to_string()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_games_returns_empty_for_nonexistent_dir() {
        let result = list_games("/tmp/nonexistent_reeln_test_dir_xyz".into()).unwrap();
        assert!(result.is_empty());
    }

    // ── get_game_state ─────────────────────────────────────────────────

    #[test]
    fn get_game_state_loads_valid_state() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        let original = crate::test_utils::create_test_game(&game_dir);

        let loaded = get_game_state(game_dir.display().to_string()).unwrap();
        assert_eq!(loaded.game_info.home_team, original.game_info.home_team);
        assert_eq!(loaded.game_info.sport, "soccer");
    }

    #[test]
    fn get_game_state_errors_for_missing_dir() {
        let err = get_game_state("/tmp/nonexistent_reeln_game_xyz".into()).unwrap_err();
        assert!(!err.is_empty());
    }

    // ── set_game_tournament ────────────────────────────────────────────

    #[test]
    fn set_game_tournament_updates_and_persists() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        crate::test_utils::create_test_game(&game_dir);

        let result =
            set_game_tournament(game_dir.display().to_string(), "Cup 2026".into()).unwrap();
        assert_eq!(result.game_info.tournament, "Cup 2026");

        let reloaded = reeln_state::load_game_state(&game_dir).unwrap();
        assert_eq!(reloaded.game_info.tournament, "Cup 2026");
    }

    // ── bulk_update_event_type ─────────────────────────────────────────

    #[test]
    fn bulk_update_event_type_updates_matching_events() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        setup_game_with_events(
            &game_dir,
            vec![
                make_event("evt1", "a.mp4", "goal"),
                make_event("evt2", "b.mp4", "goal"),
                make_event("evt3", "c.mp4", "save"),
            ],
        );

        let result = bulk_update_event_type(
            game_dir.display().to_string(),
            vec!["evt1".into(), "evt2".into()],
            "penalty".into(),
        )
        .unwrap();

        assert_eq!(result.events[0].event_type, "penalty");
        assert_eq!(result.events[1].event_type, "penalty");
        assert_eq!(result.events[2].event_type, "save"); // unchanged
    }

    #[test]
    fn bulk_update_event_type_ignores_nonexistent_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        setup_game_with_events(&game_dir, vec![make_event("evt1", "a.mp4", "goal")]);

        let result = bulk_update_event_type(
            game_dir.display().to_string(),
            vec!["evt1".into(), "nonexistent".into()],
            "assist".into(),
        )
        .unwrap();

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].event_type, "assist");
    }

    // ── quick_tag_event ────────────────────────────────────────────────

    #[test]
    fn quick_tag_event_sets_type_and_team() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        setup_game_with_events(&game_dir, vec![make_event("evt1", "a.mp4", "")]);

        let result = quick_tag_event(
            game_dir.display().to_string(),
            "evt1".into(),
            "goal".into(),
            Some("home".into()),
        )
        .unwrap();

        assert_eq!(result.events[0].event_type, "goal");
        assert_eq!(
            result.events[0].metadata.get("team"),
            Some(&serde_json::Value::String("home".to_string()))
        );
    }

    #[test]
    fn quick_tag_event_without_team_removes_team_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        let mut evt = make_event("evt1", "a.mp4", "goal");
        evt.metadata.insert(
            "team".to_string(),
            serde_json::Value::String("home".to_string()),
        );
        setup_game_with_events(&game_dir, vec![evt]);

        let result = quick_tag_event(
            game_dir.display().to_string(),
            "evt1".into(),
            "save".into(),
            None,
        )
        .unwrap();

        assert_eq!(result.events[0].event_type, "save");
        assert!(!result.events[0].metadata.contains_key("team"));
    }

    #[test]
    fn quick_tag_event_returns_error_for_missing_event() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        setup_game_with_events(&game_dir, vec![make_event("evt1", "a.mp4", "goal")]);

        let err = quick_tag_event(
            game_dir.display().to_string(),
            "missing".into(),
            "goal".into(),
            None,
        )
        .unwrap_err();

        assert!(err.contains("missing"));
        assert!(err.contains("not found"));
    }

    // ── prune_renders ──────────────────────────────────────────────────

    #[test]
    fn prune_renders_removes_files_and_clears_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        let mut state = crate::test_utils::create_test_game(&game_dir);

        // Create actual render files on disk
        let render_file = game_dir.join("render1.mp4");
        std::fs::write(&render_file, b"fake render content 12345").unwrap();

        state.renders.push(reeln_state::RenderEntry {
            input: "clip.mp4".to_string(),
            output: render_file.display().to_string(),
            segment_number: 0,
            format: "tiktok".to_string(),
            crop_mode: "".to_string(),
            rendered_at: chrono::Utc::now().to_rfc3339(),
            event_id: "".to_string(),
        });
        reeln_state::save_game_state(&state, &game_dir).unwrap();

        let result = prune_renders(game_dir.display().to_string()).unwrap();

        // File should be deleted
        assert!(!render_file.exists());

        // JSON response should report stats
        assert_eq!(result["removed_files"], 1);
        assert_eq!(result["cleared_entries"], 1);
        assert_eq!(result["bytes_freed"], 25); // b"fake render content 12345".len()

        // Game state should have empty renders
        let reloaded = reeln_state::load_game_state(&game_dir).unwrap();
        assert!(reloaded.renders.is_empty());
    }

    #[test]
    fn prune_renders_with_no_renders() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        crate::test_utils::create_test_game(&game_dir);

        let result = prune_renders(game_dir.display().to_string()).unwrap();
        assert_eq!(result["removed_files"], 0);
        assert_eq!(result["cleared_entries"], 0);
        assert_eq!(result["bytes_freed"], 0);
    }

    #[test]
    fn prune_renders_handles_missing_render_file_gracefully() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        let mut state = crate::test_utils::create_test_game(&game_dir);

        // Add render entry pointing to a file that doesn't exist
        state.renders.push(reeln_state::RenderEntry {
            input: "clip.mp4".to_string(),
            output: game_dir.join("ghost.mp4").display().to_string(),
            segment_number: 0,
            format: "tiktok".to_string(),
            crop_mode: "".to_string(),
            rendered_at: chrono::Utc::now().to_rfc3339(),
            event_id: "".to_string(),
        });
        reeln_state::save_game_state(&state, &game_dir).unwrap();

        let result = prune_renders(game_dir.display().to_string()).unwrap();
        // Entry cleared even though file was already gone
        assert_eq!(result["cleared_entries"], 1);
        assert_eq!(result["removed_files"], 0);
    }

    // ── finish_game (command wrapper) ──────────────────────────────────

    #[test]
    fn finish_game_sets_finished_and_timestamp() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        crate::test_utils::create_test_game(&game_dir);

        let result = finish_game(game_dir.display().to_string()).unwrap();

        assert_eq!(result["finished"], true);
        assert!(!result["finished_at"].as_str().unwrap().is_empty());

        // Verify persisted
        let reloaded = reeln_state::load_game_state(&game_dir).unwrap();
        assert!(reloaded.finished);
        assert!(!reloaded.finished_at.is_empty());
    }
}
