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
    reeln_state::update_event_field(&mut state, &event_id, &field, value)
        .map_err(|e| e.to_string())?;
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
    reeln_state::set_tournament(&mut state, &tournament);
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
    description: Option<String>,
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
            description,
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
    reeln_state::bulk_update_event_type(&mut state, &event_ids, &event_type);
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

    let total = reeln_state::clear_renders(&mut state);
    reeln_state::save_game_state(&state, path).map_err(|e| e.to_string())?;

    serde_json::to_value(&serde_json::json!({
        "state": state,
        "removed_files": removed,
        "cleared_entries": total,
        "bytes_freed": bytes_freed,
    }))
    .map_err(|e| e.to_string())
}

// ── Game prune (CLI-parity) ──────────────────────────────────────

const VIDEO_EXTENSIONS: &[&str] = &[".mkv", ".mp4", ".mov", ".avi", ".webm", ".ts", ".m4v", ".flv"];
const TEMP_EXTENSIONS: &[&str] = &[".tmp", ".txt"];

/// Result of a prune dry-run or execution.
#[derive(serde::Serialize)]
pub struct PrunePreview {
    /// Files that would be / were removed, with relative paths and sizes.
    pub files: Vec<PruneFileEntry>,
    pub total_bytes: u64,
    pub file_count: u32,
    /// Whether the game is eligible for pruning (must be finished).
    pub eligible: bool,
    /// Reason if not eligible.
    pub reason: String,
}

#[derive(serde::Serialize)]
pub struct PruneFileEntry {
    pub path: String,
    pub bytes: u64,
}

/// Collect files that would be removed by pruning — dry-run mode.
/// Matches reeln-cli `prune_game(dry_run=True)` behavior exactly.
#[tauri::command]
pub fn prune_game_preview(
    game_dir: String,
    all_files: bool,
    force: Option<bool>,
) -> Result<PrunePreview, String> {
    let dir = Path::new(&game_dir);
    let state = reeln_state::load_game_state(dir).map_err(|e| e.to_string())?;

    if !state.finished {
        return Ok(PrunePreview {
            files: vec![],
            total_bytes: 0,
            file_count: 0,
            eligible: false,
            reason: "Game must be finished before pruning".to_string(),
        });
    }

    let tagged_clips: std::collections::HashSet<String> = state
        .events
        .iter()
        .filter(|e| !e.event_type.is_empty())
        .map(|e| e.clip.clone())
        .collect();
    let all_event_clips: std::collections::HashSet<String> = state
        .events
        .iter()
        .map(|e| e.clip.clone())
        .collect();
    let force = force.unwrap_or(false);

    let mut files = Vec::new();
    let mut total_bytes = 0u64;

    // Collect pruneable files
    collect_pruneable_files(dir, dir, &tagged_clips, &all_event_clips, all_files, force, &mut files, &mut total_bytes);

    // Debug directory
    let debug_path = dir.join("debug");
    if debug_path.is_dir() {
        collect_all_files_recursive(&debug_path, dir, &mut files, &mut total_bytes);
    }

    let file_count = files.len() as u32;

    Ok(PrunePreview {
        files,
        total_bytes,
        file_count,
        eligible: true,
        reason: String::new(),
    })
}

/// Execute the actual prune — removes files and reloads game state.
#[tauri::command]
pub fn prune_game_execute(
    game_dir: String,
    all_files: bool,
    force: Option<bool>,
) -> Result<PrunePreview, String> {
    let dir = Path::new(&game_dir);
    let state = reeln_state::load_game_state(dir).map_err(|e| e.to_string())?;

    if !state.finished {
        return Err("Game must be finished before pruning".to_string());
    }

    let tagged_clips: std::collections::HashSet<String> = state
        .events
        .iter()
        .filter(|e| !e.event_type.is_empty())
        .map(|e| e.clip.clone())
        .collect();
    let all_event_clips: std::collections::HashSet<String> = state
        .events
        .iter()
        .map(|e| e.clip.clone())
        .collect();
    let force = force.unwrap_or(false);

    let mut files = Vec::new();
    let mut total_bytes = 0u64;

    // Collect and remove files
    collect_pruneable_files(dir, dir, &tagged_clips, &all_event_clips, all_files, force, &mut files, &mut total_bytes);

    // Debug directory
    let debug_path = dir.join("debug");
    if debug_path.is_dir() {
        collect_all_files_recursive(&debug_path, dir, &mut files, &mut total_bytes);
    }

    // Actually delete
    for entry in &files {
        let full_path = dir.join(&entry.path);
        let _ = std::fs::remove_file(&full_path);
    }

    // Clean up debug directory
    if debug_path.is_dir() {
        remove_empty_dirs_recursive(&debug_path);
    }

    // Clean up empty segment directories
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let _ = std::fs::remove_dir(&p); // Only removes if empty
            }
        }
    }

    let file_count = files.len() as u32;

    Ok(PrunePreview {
        files,
        total_bytes,
        file_count,
        eligible: true,
        reason: String::new(),
    })
}

fn collect_pruneable_files(
    base_dir: &Path,
    scan_dir: &Path,
    tagged_clips: &std::collections::HashSet<String>,
    all_event_clips: &std::collections::HashSet<String>,
    all_files: bool,
    force: bool,
    files: &mut Vec<PruneFileEntry>,
    total_bytes: &mut u64,
) {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    collect_files_sorted(scan_dir, &mut paths);

    for path in paths {
        if let Ok(rel) = path.strip_prefix(base_dir) {
            let rel_str = rel.to_string_lossy().to_string();

            // Never remove game.json
            if rel_str == "game.json" {
                continue;
            }

            // Skip debug directory (handled separately)
            if rel_str.starts_with("debug/") || rel_str.starts_with("debug\\") {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_lowercase()))
                .unwrap_or_default();

            // Temp files always removed
            if TEMP_EXTENSIONS.contains(&ext.as_str()) {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                *total_bytes += size;
                files.push(PruneFileEntry { path: rel_str, bytes: size });
                continue;
            }

            // Only consider video files
            if !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }

            // --all: remove everything including tagged clips
            if all_files {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                *total_bytes += size;
                files.push(PruneFileEntry { path: rel_str, bytes: size });
                continue;
            }

            let is_tagged = tagged_clips.contains(&rel_str);
            let is_event_clip = all_event_clips.contains(&rel_str);

            // Tagged clips are preserved unless --all
            if is_tagged {
                continue;
            }

            // Untagged event clips require --force
            if is_event_clip && !force {
                continue;
            }

            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            *total_bytes += size;
            files.push(PruneFileEntry { path: rel_str, bytes: size });
        }
    }
}

fn collect_files_sorted(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut children: Vec<std::path::PathBuf> = entries.flatten().map(|e| e.path()).collect();
        children.sort();
        for child in children {
            if child.is_file() {
                out.push(child);
            } else if child.is_dir() {
                collect_files_sorted(&child, out);
            }
        }
    }
}

fn collect_all_files_recursive(
    dir: &Path,
    base_dir: &Path,
    files: &mut Vec<PruneFileEntry>,
    total_bytes: &mut u64,
) {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    collect_files_sorted(dir, &mut paths);
    for path in paths {
        if let Ok(rel) = path.strip_prefix(base_dir) {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            *total_bytes += size;
            files.push(PruneFileEntry {
                path: rel.to_string_lossy().to_string(),
                bytes: size,
            });
        }
    }
}

fn remove_empty_dirs_recursive(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                remove_empty_dirs_recursive(&p);
                let _ = std::fs::remove_dir(&p);
            }
        }
    }
    let _ = std::fs::remove_dir(dir);
}

// ── Game delete ─────────────────────────────────────────────────

/// Preview what deleting a game would remove — total size and file count.
#[derive(Debug, serde::Serialize)]
pub struct DeletePreview {
    pub dir_path: String,
    pub home_team: String,
    pub away_team: String,
    pub date: String,
    pub total_bytes: u64,
    pub file_count: u32,
}

#[tauri::command]
pub fn delete_game_preview(game_dir: String) -> Result<DeletePreview, String> {
    let dir = Path::new(&game_dir);
    let state = reeln_state::load_game_state(dir).map_err(|e| e.to_string())?;

    let (total_bytes, file_count) = dir_size_recursive(dir);

    Ok(DeletePreview {
        dir_path: game_dir,
        home_team: state.game_info.home_team,
        away_team: state.game_info.away_team,
        date: state.game_info.date,
        total_bytes,
        file_count,
    })
}

/// Permanently delete an entire game directory.
#[tauri::command]
pub fn delete_game(game_dir: String) -> Result<(), String> {
    let dir = Path::new(&game_dir);
    if !dir.join("game.json").exists() {
        return Err(format!("Not a game directory: {game_dir}"));
    }
    std::fs::remove_dir_all(dir).map_err(|e| format!("Failed to delete game: {e}"))
}

fn dir_size_recursive(dir: &Path) -> (u64, u32) {
    let mut total_bytes = 0u64;
    let mut file_count = 0u32;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                file_count += 1;
                total_bytes += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            } else if path.is_dir() {
                let (bytes, count) = dir_size_recursive(&path);
                total_bytes += bytes;
                file_count += count;
            }
        }
    }
    (total_bytes, file_count)
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
    reeln_state::tag_event(&mut state, &event_id, &event_type, team.as_deref())
        .map_err(|e| e.to_string())?;
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

    // ── delete_game_preview ─────────────────────────────────────────────

    #[test]
    fn delete_game_preview_returns_game_info_and_size() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        crate::test_utils::create_test_game(&game_dir);
        std::fs::write(game_dir.join("clip.mp4"), b"video content here").unwrap();

        let result = delete_game_preview(game_dir.display().to_string()).unwrap();
        assert_eq!(result.home_team, "Team A");
        assert_eq!(result.away_team, "Team B");
        assert_eq!(result.date, "2026-04-03");
        assert!(result.total_bytes > 0);
        assert!(result.file_count >= 2); // game.json + clip.mp4
    }

    #[test]
    fn delete_game_preview_errors_for_missing_dir() {
        let err = delete_game_preview("/tmp/nonexistent_reeln_delete_xyz".into()).unwrap_err();
        assert!(!err.is_empty());
    }

    // ── delete_game ───────────────────────────────────────────────────

    #[test]
    fn delete_game_removes_entire_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        crate::test_utils::create_test_game(&game_dir);
        std::fs::write(game_dir.join("clip.mp4"), b"video").unwrap();
        let sub = game_dir.join("segment1");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("raw.mkv"), b"raw video").unwrap();

        assert!(game_dir.exists());
        delete_game(game_dir.display().to_string()).unwrap();
        assert!(!game_dir.exists());
    }

    #[test]
    fn delete_game_rejects_non_game_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let not_game = tmp.path().join("random");
        std::fs::create_dir_all(&not_game).unwrap();

        let err = delete_game(not_game.display().to_string()).unwrap_err();
        assert!(err.contains("Not a game directory"));
    }

    #[test]
    fn delete_game_errors_for_missing_dir() {
        let err = delete_game("/tmp/nonexistent_reeln_delete_xyz".into()).unwrap_err();
        assert!(err.contains("Not a game directory"));
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

    // ── prune_game_preview / prune_game_execute ─────────────────────

    fn make_finished_game(game_dir: &Path) -> reeln_state::GameState {
        let mut state = crate::test_utils::create_test_game(game_dir);
        state.finished = true;
        state.finished_at = chrono::Utc::now().to_rfc3339();
        state.events.push(reeln_state::GameEvent {
            id: "e1".to_string(),
            clip: "clip_001.mkv".to_string(),
            segment_number: 1,
            event_type: "goal".to_string(),
            player: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        });
        reeln_state::save_game_state(&state, game_dir).unwrap();
        state
    }

    #[test]
    fn prune_preview_not_finished_returns_ineligible() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        crate::test_utils::create_test_game(&game_dir);

        let result = prune_game_preview(game_dir.display().to_string(), false, None).unwrap();
        assert!(!result.eligible);
        assert!(result.reason.contains("finished"));
        assert_eq!(result.file_count, 0);
    }

    #[test]
    fn prune_preview_finished_game_finds_generated_files() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        make_finished_game(&game_dir);

        // Create source clip (should be preserved)
        std::fs::write(game_dir.join("clip_001.mkv"), b"source clip").unwrap();
        // Create generated files (should be pruned)
        std::fs::write(game_dir.join("highlight.mp4"), b"highlight reel content").unwrap();
        std::fs::write(game_dir.join("concat.tmp"), b"temp").unwrap();

        let result = prune_game_preview(game_dir.display().to_string(), false, None).unwrap();
        assert!(result.eligible);
        assert_eq!(result.file_count, 2); // highlight.mp4 + concat.tmp
        assert!(result.files.iter().any(|f| f.path == "highlight.mp4"));
        assert!(result.files.iter().any(|f| f.path == "concat.tmp"));
        // Source clip should NOT be in the list
        assert!(!result.files.iter().any(|f| f.path == "clip_001.mkv"));
    }

    #[test]
    fn prune_preview_all_files_includes_source_clips() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        make_finished_game(&game_dir);

        std::fs::write(game_dir.join("clip_001.mkv"), b"source clip").unwrap();
        std::fs::write(game_dir.join("highlight.mp4"), b"highlight").unwrap();

        let result = prune_game_preview(game_dir.display().to_string(), true, None).unwrap();
        assert!(result.eligible);
        assert_eq!(result.file_count, 2); // both files
        assert!(result.files.iter().any(|f| f.path == "clip_001.mkv"));
    }

    #[test]
    fn prune_preview_includes_debug_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        make_finished_game(&game_dir);

        let debug_dir = game_dir.join("debug");
        std::fs::create_dir_all(&debug_dir).unwrap();
        std::fs::write(debug_dir.join("log.txt"), b"debug log").unwrap();

        let result = prune_game_preview(game_dir.display().to_string(), false, None).unwrap();
        assert!(result.files.iter().any(|f| f.path.contains("debug")));
    }

    #[test]
    fn prune_preview_nothing_to_prune() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        make_finished_game(&game_dir);
        // Only source clip and game.json — nothing to prune
        std::fs::write(game_dir.join("clip_001.mkv"), b"source").unwrap();

        let result = prune_game_preview(game_dir.display().to_string(), false, None).unwrap();
        assert!(result.eligible);
        assert_eq!(result.file_count, 0);
    }

    #[test]
    fn prune_execute_removes_files() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        make_finished_game(&game_dir);

        std::fs::write(game_dir.join("clip_001.mkv"), b"source clip").unwrap();
        std::fs::write(game_dir.join("highlight.mp4"), b"highlight reel").unwrap();
        std::fs::write(game_dir.join("concat.tmp"), b"temp file").unwrap();

        let result = prune_game_execute(game_dir.display().to_string(), false, None).unwrap();
        assert_eq!(result.file_count, 2);
        // Generated files deleted
        assert!(!game_dir.join("highlight.mp4").exists());
        assert!(!game_dir.join("concat.tmp").exists());
        // Source clip preserved
        assert!(game_dir.join("clip_001.mkv").exists());
        // game.json preserved
        assert!(game_dir.join("game.json").exists());
    }

    #[test]
    fn prune_execute_not_finished_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        crate::test_utils::create_test_game(&game_dir);

        let result = prune_game_execute(game_dir.display().to_string(), false, None);
        assert!(result.is_err());
    }

    #[test]
    fn prune_preview_force_includes_untagged_clips() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        let mut state = crate::test_utils::create_test_game(&game_dir);
        state.finished = true;
        state.finished_at = chrono::Utc::now().to_rfc3339();
        // Tagged event clip
        state.events.push(reeln_state::GameEvent {
            id: "e1".to_string(),
            clip: "tagged.mkv".to_string(),
            segment_number: 1,
            event_type: "goal".to_string(),
            player: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        });
        // Untagged event clip (empty event_type)
        state.events.push(reeln_state::GameEvent {
            id: "e2".to_string(),
            clip: "untagged.mkv".to_string(),
            segment_number: 1,
            event_type: String::new(),
            player: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        });
        reeln_state::save_game_state(&state, &game_dir).unwrap();

        std::fs::write(game_dir.join("tagged.mkv"), b"tagged clip").unwrap();
        std::fs::write(game_dir.join("untagged.mkv"), b"untagged clip").unwrap();

        // Without force: untagged clip is preserved
        let result = prune_game_preview(game_dir.display().to_string(), false, None).unwrap();
        assert!(!result.files.iter().any(|f| f.path == "untagged.mkv"));
        assert!(!result.files.iter().any(|f| f.path == "tagged.mkv"));

        // With force: untagged clip is included for removal
        let result = prune_game_preview(game_dir.display().to_string(), false, Some(true)).unwrap();
        assert!(result.files.iter().any(|f| f.path == "untagged.mkv"));
        assert!(!result.files.iter().any(|f| f.path == "tagged.mkv"));
    }

    #[test]
    fn prune_execute_force_removes_untagged_clips() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("game1");
        let mut state = crate::test_utils::create_test_game(&game_dir);
        state.finished = true;
        state.finished_at = chrono::Utc::now().to_rfc3339();
        state.events.push(reeln_state::GameEvent {
            id: "e1".to_string(),
            clip: "tagged.mkv".to_string(),
            segment_number: 1,
            event_type: "goal".to_string(),
            player: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        });
        state.events.push(reeln_state::GameEvent {
            id: "e2".to_string(),
            clip: "untagged.mkv".to_string(),
            segment_number: 1,
            event_type: String::new(),
            player: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        });
        reeln_state::save_game_state(&state, &game_dir).unwrap();

        std::fs::write(game_dir.join("tagged.mkv"), b"tagged").unwrap();
        std::fs::write(game_dir.join("untagged.mkv"), b"untagged").unwrap();

        let result = prune_game_execute(game_dir.display().to_string(), false, Some(true)).unwrap();
        assert!(result.files.iter().any(|f| f.path == "untagged.mkv"));
        assert!(!game_dir.join("untagged.mkv").exists());
        // Tagged clip preserved
        assert!(game_dir.join("tagged.mkv").exists());
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
