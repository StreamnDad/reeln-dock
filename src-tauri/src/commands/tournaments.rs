use std::collections::HashMap;
use std::path::PathBuf;

use tauri::State;

use crate::models::TournamentMeta;
use crate::state::AppState;

/// Return the path to the tournaments metadata file in the output directory.
fn tournaments_file(state: &AppState) -> Result<PathBuf, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let config = config.as_ref().ok_or("No config loaded")?;
    let output_dir = config
        .paths
        .output_dir
        .as_deref()
        .ok_or("No output directory configured")?;
    Ok(PathBuf::from(output_dir).join("tournaments.json"))
}

fn load_meta_map(state: &AppState) -> Result<HashMap<String, TournamentMeta>, String> {
    let path = tournaments_file(state)?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_meta_map(
    state: &AppState,
    map: &HashMap<String, TournamentMeta>,
) -> Result<(), String> {
    let path = tournaments_file(state)?;
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// List all tournament metadata. Returns entries for tournaments that have metadata.
#[tauri::command]
pub fn list_tournament_metadata(
    state: State<'_, AppState>,
) -> Result<Vec<TournamentMeta>, String> {
    let map = load_meta_map(&state)?;
    let mut list: Vec<TournamentMeta> = map.into_values().collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}

/// Set a tournament's archived status.
#[tauri::command]
pub fn set_tournament_archived(
    name: String,
    archived: bool,
    state: State<'_, AppState>,
) -> Result<TournamentMeta, String> {
    let mut map = load_meta_map(&state)?;
    let entry = map.entry(name.clone()).or_insert_with(|| TournamentMeta {
        name: name.clone(),
        archived: false,
        notes: String::new(),
        start_date: String::new(),
        end_date: String::new(),
        url: String::new(),
    });
    entry.archived = archived;
    let result = entry.clone();
    save_meta_map(&state, &map)?;
    Ok(result)
}

/// Update tournament metadata (notes, etc.).
#[tauri::command]
pub fn update_tournament_metadata(
    meta: TournamentMeta,
    state: State<'_, AppState>,
) -> Result<TournamentMeta, String> {
    let mut map = load_meta_map(&state)?;
    map.insert(meta.name.clone(), meta.clone());
    save_meta_map(&state, &map)?;
    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, DockSettings};
    use crate::test_utils;
    use reeln_sport::SportRegistry;
    use std::sync::Mutex;

    // ── Helper ──────────────────────────────────────────────────────

    fn make_state_with_output(dir: &tempfile::TempDir) -> (AppState, PathBuf) {
        let output_dir = dir.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        let mut config = reeln_config::AppConfig::default();
        config.paths.output_dir = Some(output_dir.clone());

        let state = AppState {
            config: Mutex::new(Some(config)),
            sport_registry: Mutex::new(SportRegistry::default()),
            dock_settings: Mutex::new(DockSettings::default()),
            app_data_dir: dir.path().to_path_buf(),
            media_backend: test_utils::mock_backend(),
            auth_child_pid: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        (state, output_dir)
    }

    fn make_meta(name: &str) -> TournamentMeta {
        TournamentMeta {
            name: name.to_string(),
            archived: false,
            notes: String::new(),
            start_date: String::new(),
            end_date: String::new(),
            url: String::new(),
        }
    }

    // ── tournaments_file ────────────────────────────────────────────

    #[test]
    fn tournaments_file_returns_correct_path() {
        let dir = tempfile::tempdir().unwrap();
        let (state, output_dir) = make_state_with_output(&dir);
        let path = tournaments_file(&state).unwrap();
        assert_eq!(path, output_dir.join("tournaments.json"));
    }

    #[test]
    fn tournaments_file_errors_when_no_config() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState {
            config: Mutex::new(None),
            sport_registry: Mutex::new(SportRegistry::default()),
            dock_settings: Mutex::new(DockSettings::default()),
            app_data_dir: dir.path().to_path_buf(),
            media_backend: test_utils::mock_backend(),
            auth_child_pid: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let result = tournaments_file(&state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No config loaded"));
    }

    #[test]
    fn tournaments_file_errors_when_no_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = reeln_config::AppConfig::default(); // output_dir is None
        let state = AppState {
            config: Mutex::new(Some(config)),
            sport_registry: Mutex::new(SportRegistry::default()),
            dock_settings: Mutex::new(DockSettings::default()),
            app_data_dir: dir.path().to_path_buf(),
            media_backend: test_utils::mock_backend(),
            auth_child_pid: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let result = tournaments_file(&state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No output directory"));
    }

    // ── load_meta_map ───────────────────────────────────────────────

    #[test]
    fn load_meta_map_empty_when_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let (state, _) = make_state_with_output(&dir);
        let map = load_meta_map(&state).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn load_meta_map_loads_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let (state, output_dir) = make_state_with_output(&dir);

        let mut map = HashMap::new();
        map.insert(
            "Spring Cup".to_string(),
            TournamentMeta {
                name: "Spring Cup".to_string(),
                archived: false,
                notes: "A great tournament".to_string(),
                start_date: "2026-03-01".to_string(),
                end_date: "2026-03-15".to_string(),
                url: "https://example.com".to_string(),
            },
        );
        let json = serde_json::to_string_pretty(&map).unwrap();
        std::fs::write(output_dir.join("tournaments.json"), json).unwrap();

        let loaded = load_meta_map(&state).unwrap();
        assert_eq!(loaded.len(), 1);
        let meta = loaded.get("Spring Cup").unwrap();
        assert_eq!(meta.name, "Spring Cup");
        assert_eq!(meta.notes, "A great tournament");
        assert_eq!(meta.start_date, "2026-03-01");
    }

    // ── save_meta_map ───────────────────────────────────────────────

    #[test]
    fn save_meta_map_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let (state, output_dir) = make_state_with_output(&dir);

        let mut map = HashMap::new();
        map.insert("Fall Classic".to_string(), make_meta("Fall Classic"));
        save_meta_map(&state, &map).unwrap();

        let path = output_dir.join("tournaments.json");
        assert!(path.is_file());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let (state, _) = make_state_with_output(&dir);

        let mut map = HashMap::new();
        map.insert("T1".to_string(), make_meta("T1"));
        map.insert(
            "T2".to_string(),
            TournamentMeta {
                name: "T2".to_string(),
                archived: true,
                notes: "Archived tourney".to_string(),
                start_date: "2026-01-01".to_string(),
                end_date: "2026-01-31".to_string(),
                url: "https://t2.example.com".to_string(),
            },
        );

        save_meta_map(&state, &map).unwrap();
        let loaded = load_meta_map(&state).unwrap();

        assert_eq!(loaded.len(), 2);
        assert!(!loaded["T1"].archived);
        assert!(loaded["T2"].archived);
        assert_eq!(loaded["T2"].notes, "Archived tourney");
        assert_eq!(loaded["T2"].url, "https://t2.example.com");
    }

    // ── Integration-style tests for the metadata operations ─────────

    #[test]
    fn set_archived_creates_entry_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let (state, _) = make_state_with_output(&dir);

        // Simulate what set_tournament_archived does
        let mut map = load_meta_map(&state).unwrap();
        let entry = map
            .entry("New Tourney".to_string())
            .or_insert_with(|| make_meta("New Tourney"));
        entry.archived = true;
        save_meta_map(&state, &map).unwrap();

        let reloaded = load_meta_map(&state).unwrap();
        assert!(reloaded["New Tourney"].archived);
    }

    #[test]
    fn update_metadata_overwrites_entry() {
        let dir = tempfile::tempdir().unwrap();
        let (state, _) = make_state_with_output(&dir);

        // Insert initial
        let mut map = HashMap::new();
        map.insert("Tourney".to_string(), make_meta("Tourney"));
        save_meta_map(&state, &map).unwrap();

        // Update (like update_tournament_metadata)
        let mut map = load_meta_map(&state).unwrap();
        map.insert(
            "Tourney".to_string(),
            TournamentMeta {
                name: "Tourney".to_string(),
                archived: false,
                notes: "Updated notes".to_string(),
                start_date: "2026-06-01".to_string(),
                end_date: "2026-06-30".to_string(),
                url: "https://updated.example.com".to_string(),
            },
        );
        save_meta_map(&state, &map).unwrap();

        let reloaded = load_meta_map(&state).unwrap();
        assert_eq!(reloaded["Tourney"].notes, "Updated notes");
        assert_eq!(reloaded["Tourney"].url, "https://updated.example.com");
    }

    #[test]
    fn list_metadata_sorted_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let (state, _) = make_state_with_output(&dir);

        let mut map = HashMap::new();
        map.insert("Zulu Cup".to_string(), make_meta("Zulu Cup"));
        map.insert("Alpha League".to_string(), make_meta("Alpha League"));
        map.insert("Mike Open".to_string(), make_meta("Mike Open"));
        save_meta_map(&state, &map).unwrap();

        let loaded = load_meta_map(&state).unwrap();
        let mut list: Vec<TournamentMeta> = loaded.into_values().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));

        let names: Vec<&str> = list.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha League", "Mike Open", "Zulu Cup"]);
    }
}
