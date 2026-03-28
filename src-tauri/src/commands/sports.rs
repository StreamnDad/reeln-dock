use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub fn list_sports(state: State<'_, AppState>) -> Result<Vec<reeln_sport::SportAlias>, String> {
    let registry = state.sport_registry.lock().map_err(|e| e.to_string())?;
    Ok(registry.list_sports().into_iter().cloned().collect())
}
