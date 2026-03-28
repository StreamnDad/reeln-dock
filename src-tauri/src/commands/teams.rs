use std::path::PathBuf;

use tauri::State;

use crate::models::TeamProfile;
use crate::state::AppState;

/// Return the teams base directory, derived from the effective config dir.
fn teams_base_dir(state: &AppState) -> PathBuf {
    state.effective_config_dir().join("teams")
}

/// Slugify a team name to a filesystem-safe string (matches Python CLI).
fn slugify(name: &str) -> String {
    let lower = name.to_lowercase();
    let slug: String = lower
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    slug.trim_matches('_').to_string()
}

#[tauri::command]
pub fn list_team_levels(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let base = teams_base_dir(&state);
    if !base.is_dir() {
        return Ok(Vec::new());
    }
    let mut levels: Vec<String> = std::fs::read_dir(&base)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    levels.sort();
    Ok(levels)
}

#[tauri::command]
pub fn list_team_profiles(
    level: String,
    state: State<'_, AppState>,
) -> Result<Vec<TeamProfile>, String> {
    let level_dir = teams_base_dir(&state).join(&level);
    if !level_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut profiles = Vec::new();
    for entry in std::fs::read_dir(&level_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") && path.is_file() {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            match serde_json::from_str::<TeamProfile>(&content) {
                Ok(mut profile) => {
                    if profile.level.is_empty() {
                        profile.level = level.clone();
                    }
                    profiles.push(profile);
                }
                Err(_) => continue,
            }
        }
    }
    profiles.sort_by(|a, b| a.team_name.cmp(&b.team_name));
    Ok(profiles)
}

#[tauri::command]
pub fn save_team_profile(
    profile: TeamProfile,
    state: State<'_, AppState>,
) -> Result<TeamProfile, String> {
    let slug = slugify(&profile.team_name);
    let dest = teams_base_dir(&state)
        .join(&profile.level)
        .join(format!("{slug}.json"));
    std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    std::fs::write(&dest, json).map_err(|e| e.to_string())?;
    Ok(profile)
}
