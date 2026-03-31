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
