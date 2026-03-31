use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::State;

use crate::models::TeamProfile;
use crate::state::AppState;

/// A single player entry parsed from a roster CSV.
#[derive(Debug, Clone, Serialize)]
pub struct RosterEntry {
    pub number: String,
    pub name: String,
}

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

#[tauri::command]
pub fn delete_team_profile(
    level: String,
    team_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let slug = slugify(&team_name);
    let path = teams_base_dir(&state)
        .join(&level)
        .join(format!("{slug}.json"));
    if !path.is_file() {
        return Err(format!("Team profile not found: {level}/{team_name}"));
    }
    std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn clone_team_profile(
    source_level: String,
    source_name: String,
    new_name: String,
    new_level: Option<String>,
    state: State<'_, AppState>,
) -> Result<TeamProfile, String> {
    let source_slug = slugify(&source_name);
    let source_path = teams_base_dir(&state)
        .join(&source_level)
        .join(format!("{source_slug}.json"));
    if !source_path.is_file() {
        return Err(format!("Source team not found: {source_level}/{source_name}"));
    }
    let content = std::fs::read_to_string(&source_path).map_err(|e| e.to_string())?;
    let mut profile: TeamProfile =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;
    profile.team_name = new_name;
    profile.level = new_level.unwrap_or(source_level);
    let dest_slug = slugify(&profile.team_name);
    let dest = teams_base_dir(&state)
        .join(&profile.level)
        .join(format!("{dest_slug}.json"));
    if dest.is_file() {
        return Err(format!("Team '{}' already exists in level '{}'", profile.team_name, profile.level));
    }
    std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    std::fs::write(&dest, json).map_err(|e| e.to_string())?;
    Ok(profile)
}

#[tauri::command]
pub fn rename_team_level(
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let base = teams_base_dir(&state);
    let old_dir = base.join(&old_name);
    let new_dir = base.join(&new_name);
    if !old_dir.is_dir() {
        return Err(format!("Level '{}' not found", old_name));
    }
    if new_dir.exists() {
        return Err(format!("Level '{}' already exists", new_name));
    }
    std::fs::rename(&old_dir, &new_dir).map_err(|e| e.to_string())?;
    // Update level field in each team profile
    for entry in std::fs::read_dir(&new_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") && path.is_file() {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            if let Ok(mut profile) = serde_json::from_str::<TeamProfile>(&content) {
                profile.level = new_name.clone();
                let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
                std::fs::write(&path, json).map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn delete_team_level(
    level: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let level_dir = teams_base_dir(&state).join(&level);
    if !level_dir.is_dir() {
        return Err(format!("Level '{}' not found", level));
    }
    // Check if level has any team profiles
    let has_teams = std::fs::read_dir(&level_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .any(|e| {
            e.path().extension().and_then(|ext| ext.to_str()) == Some("json") && e.path().is_file()
        });
    if has_teams {
        return Err(format!(
            "Cannot delete level '{}': it still contains teams. Move or delete teams first.",
            level
        ));
    }
    std::fs::remove_dir(&level_dir).map_err(|e| e.to_string())?;
    Ok(())
}

/// Load a roster CSV file and return parsed player entries.
///
/// Supports two CSV formats:
/// - Two columns: `#,Player` (number, name)
/// - Three columns: `#,Player,Position` (number, name, position — position ignored)
#[tauri::command]
pub fn load_roster(roster_path: String) -> Result<Vec<RosterEntry>, String> {
    let path = Path::new(&roster_path);
    if !path.is_file() {
        return Err(format!("Roster file not found: {roster_path}"));
    }
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Skip header row
        if i == 0 && (line.starts_with('#') || line.to_lowercase().contains("player")) {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, ',').collect();
        if parts.len() >= 2 {
            let number = parts[0].trim().to_string();
            let name = parts[1].trim().trim_matches('"').to_string();
            if !name.is_empty() {
                entries.push(RosterEntry { number, name });
            }
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Look up a team's roster by team name and level.
/// Finds the team profile, reads its roster_path, and loads the CSV.
#[tauri::command]
pub fn load_team_roster(
    team_name: String,
    level: String,
    state: State<'_, AppState>,
) -> Result<Vec<RosterEntry>, String> {
    let slug = slugify(&team_name);
    let profile_path = teams_base_dir(&state)
        .join(&level)
        .join(format!("{slug}.json"));
    if !profile_path.is_file() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&profile_path).map_err(|e| e.to_string())?;
    let profile: TeamProfile = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    if profile.roster_path.is_empty() {
        return Ok(Vec::new());
    }
    load_roster(profile.roster_path)
}
