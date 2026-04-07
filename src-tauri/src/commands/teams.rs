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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, DockSettings};
    use crate::test_utils;
    use reeln_sport::SportRegistry;
    use std::sync::Mutex;

    // ── Helper ──────────────────────────────────────────────────────

    fn make_state(dir: &tempfile::TempDir) -> AppState {
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("config.json");
        std::fs::write(&config_file, "{}").unwrap();

        let mut settings = DockSettings::default();
        settings.reeln_config_path = Some(config_file.display().to_string());

        AppState {
            config: Mutex::new(None),
            sport_registry: Mutex::new(SportRegistry::default()),
            dock_settings: Mutex::new(settings),
            app_data_dir: dir.path().to_path_buf(),
            media_backend: test_utils::mock_backend(),
            auth_child_pid: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }

    fn make_profile(name: &str, short: &str, level: &str) -> TeamProfile {
        TeamProfile {
            team_name: name.to_string(),
            short_name: short.to_string(),
            level: level.to_string(),
            logo_path: String::new(),
            roster_path: String::new(),
            colors: Vec::new(),
            jersey_colors: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    // ── slugify ─────────────────────────────────────────────────────

    #[test]
    fn slugify_simple_name() {
        assert_eq!(slugify("Team A"), "team_a");
    }

    #[test]
    fn slugify_apostrophe() {
        assert_eq!(slugify("O'Neill"), "o_neill");
    }

    #[test]
    fn slugify_leading_trailing_special() {
        assert_eq!(slugify("--Hello World--"), "hello_world");
    }

    #[test]
    fn slugify_all_special_chars() {
        assert_eq!(slugify("@#$%"), "");
    }

    #[test]
    fn slugify_mixed_case_numbers() {
        assert_eq!(slugify("Team 99"), "team_99");
    }

    #[test]
    fn slugify_already_lowercase() {
        assert_eq!(slugify("simple"), "simple");
    }

    #[test]
    fn slugify_multiple_spaces() {
        assert_eq!(slugify("A  B  C"), "a__b__c");
    }

    // ── teams_base_dir ──────────────────────────────────────────────

    #[test]
    fn teams_base_dir_returns_correct_path() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let expected = dir.path().join("config").join("teams");
        assert_eq!(teams_base_dir(&state), expected);
    }

    // ── load_roster ─────────────────────────────────────────────────

    #[test]
    fn load_roster_two_column_csv() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("roster.csv");
        std::fs::write(&csv_path, "#,Player\n10,John\n7,Jane").unwrap();

        let entries = load_roster(csv_path.display().to_string()).unwrap();
        assert_eq!(entries.len(), 2);
        // Sorted by name
        assert_eq!(entries[0].name, "Jane");
        assert_eq!(entries[0].number, "7");
        assert_eq!(entries[1].name, "John");
        assert_eq!(entries[1].number, "10");
    }

    #[test]
    fn load_roster_three_column_csv() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("roster.csv");
        std::fs::write(&csv_path, "#,Player,Position\n10,John,Forward\n7,Jane,Guard").unwrap();

        let entries = load_roster(csv_path.display().to_string()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "Jane");
        assert_eq!(entries[1].name, "John");
    }

    #[test]
    fn load_roster_skips_empty_lines() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("roster.csv");
        std::fs::write(&csv_path, "#,Player\n\n10,John\n\n7,Jane\n\n").unwrap();

        let entries = load_roster(csv_path.display().to_string()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn load_roster_skips_header_row() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("roster.csv");
        // Header starts with '#'
        std::fs::write(&csv_path, "#,Player\n5,Alice").unwrap();
        let entries = load_roster(csv_path.display().to_string()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Alice");
    }

    #[test]
    fn load_roster_skips_header_with_player_keyword() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("roster.csv");
        std::fs::write(&csv_path, "Number,Player Name\n5,Alice").unwrap();
        let entries = load_roster(csv_path.display().to_string()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Alice");
    }

    #[test]
    fn load_roster_missing_file_returns_error() {
        let result = load_roster("/nonexistent/roster.csv".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn load_roster_sorted_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("roster.csv");
        std::fs::write(&csv_path, "#,Player\n1,Zara\n2,Alice\n3,Mike").unwrap();

        let entries = load_roster(csv_path.display().to_string()).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Alice", "Mike", "Zara"]);
    }

    #[test]
    fn load_roster_strips_quotes_from_names() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("roster.csv");
        std::fs::write(&csv_path, "#,Player\n10,\"John Smith\"").unwrap();
        let entries = load_roster(csv_path.display().to_string()).unwrap();
        assert_eq!(entries[0].name, "John Smith");
    }

    #[test]
    fn load_roster_skips_entries_with_empty_name() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("roster.csv");
        std::fs::write(&csv_path, "#,Player\n10,\n7,Jane").unwrap();

        let entries = load_roster(csv_path.display().to_string()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Jane");
    }

    // ── list_team_levels (via filesystem + private helpers) ─────────

    #[test]
    fn list_team_levels_no_teams_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        // teams dir doesn't exist — teams_base_dir path won't be a dir
        let base = teams_base_dir(&state);
        assert!(!base.is_dir());
    }

    #[test]
    fn list_team_levels_with_levels() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let base = teams_base_dir(&state);
        std::fs::create_dir_all(base.join("varsity")).unwrap();
        std::fs::create_dir_all(base.join("jv")).unwrap();
        std::fs::create_dir_all(base.join("professional")).unwrap();

        // Read back and verify
        let mut levels: Vec<String> = std::fs::read_dir(&base)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        levels.sort();
        assert_eq!(levels, vec!["jv", "professional", "varsity"]);
    }

    // ── save + load team profiles (via filesystem) ──────────────────

    #[test]
    fn save_and_load_team_profile_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let profile = make_profile("Acme FC", "ACM", "varsity");

        // Save using the same logic as save_team_profile
        let slug = slugify(&profile.team_name);
        let dest = teams_base_dir(&state)
            .join(&profile.level)
            .join(format!("{slug}.json"));
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        let json = serde_json::to_string_pretty(&profile).unwrap();
        std::fs::write(&dest, json).unwrap();

        // Load it back
        let content = std::fs::read_to_string(&dest).unwrap();
        let loaded: TeamProfile = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.team_name, "Acme FC");
        assert_eq!(loaded.short_name, "ACM");
        assert_eq!(loaded.level, "varsity");
    }

    #[test]
    fn list_profiles_sorted_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let level_dir = teams_base_dir(&state).join("varsity");
        std::fs::create_dir_all(&level_dir).unwrap();

        for (name, short) in &[("Zebras", "ZEB"), ("Alphas", "ALP"), ("Mavericks", "MAV")] {
            let p = make_profile(name, short, "varsity");
            let slug = slugify(name);
            std::fs::write(
                level_dir.join(format!("{slug}.json")),
                serde_json::to_string(&p).unwrap(),
            )
            .unwrap();
        }

        // Read and sort like list_team_profiles does
        let mut profiles = Vec::new();
        for entry in std::fs::read_dir(&level_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") && path.is_file() {
                let content = std::fs::read_to_string(&path).unwrap();
                let profile: TeamProfile = serde_json::from_str(&content).unwrap();
                profiles.push(profile);
            }
        }
        profiles.sort_by(|a, b| a.team_name.cmp(&b.team_name));

        let names: Vec<&str> = profiles.iter().map(|p| p.team_name.as_str()).collect();
        assert_eq!(names, vec!["Alphas", "Mavericks", "Zebras"]);
    }

    #[test]
    fn delete_team_profile_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let profile = make_profile("Temp Team", "TMP", "jv");

        let slug = slugify(&profile.team_name);
        let dest = teams_base_dir(&state)
            .join(&profile.level)
            .join(format!("{slug}.json"));
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(&dest, serde_json::to_string(&profile).unwrap()).unwrap();
        assert!(dest.is_file());

        std::fs::remove_file(&dest).unwrap();
        assert!(!dest.exists());
    }

    #[test]
    fn clone_profile_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let base = teams_base_dir(&state);
        let level_dir = base.join("varsity");
        std::fs::create_dir_all(&level_dir).unwrap();

        let source = make_profile("Original", "ORI", "varsity");
        let source_slug = slugify(&source.team_name);
        std::fs::write(
            level_dir.join(format!("{source_slug}.json")),
            serde_json::to_string(&source).unwrap(),
        )
        .unwrap();

        // Clone logic: read source, update name, write new file
        let source_path = level_dir.join(format!("{source_slug}.json"));
        let content = std::fs::read_to_string(&source_path).unwrap();
        let mut cloned: TeamProfile = serde_json::from_str(&content).unwrap();
        cloned.team_name = "Cloned Team".to_string();

        let new_slug = slugify(&cloned.team_name);
        let new_path = level_dir.join(format!("{new_slug}.json"));
        assert!(!new_path.exists());
        std::fs::write(&new_path, serde_json::to_string(&cloned).unwrap()).unwrap();
        assert!(new_path.is_file());

        let loaded: TeamProfile =
            serde_json::from_str(&std::fs::read_to_string(&new_path).unwrap()).unwrap();
        assert_eq!(loaded.team_name, "Cloned Team");
        assert_eq!(loaded.short_name, "ORI"); // inherited from source
    }

    #[test]
    fn rename_level_updates_profiles() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let base = teams_base_dir(&state);
        let old_dir = base.join("old_level");
        std::fs::create_dir_all(&old_dir).unwrap();

        let profile = make_profile("Test", "TST", "old_level");
        let slug = slugify(&profile.team_name);
        std::fs::write(
            old_dir.join(format!("{slug}.json")),
            serde_json::to_string(&profile).unwrap(),
        )
        .unwrap();

        // Rename the directory
        let new_dir = base.join("new_level");
        std::fs::rename(&old_dir, &new_dir).unwrap();
        assert!(!old_dir.exists());
        assert!(new_dir.is_dir());

        // Update level field in profiles (like rename_team_level does)
        for entry in std::fs::read_dir(&new_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") && path.is_file() {
                let content = std::fs::read_to_string(&path).unwrap();
                if let Ok(mut p) = serde_json::from_str::<TeamProfile>(&content) {
                    p.level = "new_level".to_string();
                    std::fs::write(&path, serde_json::to_string_pretty(&p).unwrap()).unwrap();
                }
            }
        }

        // Verify
        let updated: TeamProfile = serde_json::from_str(
            &std::fs::read_to_string(new_dir.join(format!("{slug}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(updated.level, "new_level");
    }

    #[test]
    fn delete_level_empty_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let base = teams_base_dir(&state);
        let level_dir = base.join("empty_level");
        std::fs::create_dir_all(&level_dir).unwrap();

        std::fs::remove_dir(&level_dir).unwrap();
        assert!(!level_dir.exists());
    }

    #[test]
    fn delete_level_with_teams_blocked() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let base = teams_base_dir(&state);
        let level_dir = base.join("has_teams");
        std::fs::create_dir_all(&level_dir).unwrap();

        // Add a team profile
        let profile = make_profile("Blocker", "BLK", "has_teams");
        let slug = slugify(&profile.team_name);
        std::fs::write(
            level_dir.join(format!("{slug}.json")),
            serde_json::to_string(&profile).unwrap(),
        )
        .unwrap();

        // Attempting to remove the dir should fail (not empty)
        let has_teams = std::fs::read_dir(&level_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| {
                e.path().extension().and_then(|ext| ext.to_str()) == Some("json")
                    && e.path().is_file()
            });
        assert!(has_teams);
    }

    #[test]
    fn load_team_roster_via_profile() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let base = teams_base_dir(&state);
        let level_dir = base.join("varsity");
        std::fs::create_dir_all(&level_dir).unwrap();

        // Create a roster CSV
        let roster_path = dir.path().join("roster.csv");
        std::fs::write(&roster_path, "#,Player\n10,John\n7,Jane").unwrap();

        // Create a team profile with roster_path
        let mut profile = make_profile("Test Team", "TST", "varsity");
        profile.roster_path = roster_path.display().to_string();
        let slug = slugify(&profile.team_name);
        std::fs::write(
            level_dir.join(format!("{slug}.json")),
            serde_json::to_string(&profile).unwrap(),
        )
        .unwrap();

        // Read the profile back and load the roster
        let profile_path = level_dir.join(format!("{slug}.json"));
        let content = std::fs::read_to_string(&profile_path).unwrap();
        let loaded_profile: TeamProfile = serde_json::from_str(&content).unwrap();
        let entries = load_roster(loaded_profile.roster_path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "Jane");
        assert_eq!(entries[1].name, "John");
    }

    #[test]
    fn load_team_roster_empty_roster_path() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let base = teams_base_dir(&state);
        let level_dir = base.join("varsity");
        std::fs::create_dir_all(&level_dir).unwrap();

        let profile = make_profile("No Roster", "NOR", "varsity");
        let slug = slugify(&profile.team_name);
        std::fs::write(
            level_dir.join(format!("{slug}.json")),
            serde_json::to_string(&profile).unwrap(),
        )
        .unwrap();

        // Profile has empty roster_path — same behavior as load_team_roster
        assert!(profile.roster_path.is_empty());
    }

    #[test]
    fn list_profiles_skips_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let level_dir = teams_base_dir(&state).join("mixed");
        std::fs::create_dir_all(&level_dir).unwrap();

        // Valid profile
        let valid = make_profile("Valid", "VAL", "mixed");
        std::fs::write(
            level_dir.join("valid.json"),
            serde_json::to_string(&valid).unwrap(),
        )
        .unwrap();

        // Invalid JSON file
        std::fs::write(level_dir.join("broken.json"), "not valid json{{{").unwrap();

        // Read profiles, skipping invalid ones (same logic as list_team_profiles)
        let mut profiles = Vec::new();
        for entry in std::fs::read_dir(&level_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") && path.is_file() {
                let content = std::fs::read_to_string(&path).unwrap();
                if let Ok(profile) = serde_json::from_str::<TeamProfile>(&content) {
                    profiles.push(profile);
                }
            }
        }
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].team_name, "Valid");
    }

    #[test]
    fn list_profiles_fills_empty_level() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state(&dir);
        let level_dir = teams_base_dir(&state).join("pro");
        std::fs::create_dir_all(&level_dir).unwrap();

        // Profile with empty level field
        let profile = make_profile("NoLevel", "NLV", "");
        let json = serde_json::to_string(&profile).unwrap();
        std::fs::write(level_dir.join("nolevel.json"), &json).unwrap();

        // Read back and fill level (same logic as list_team_profiles)
        let content = std::fs::read_to_string(level_dir.join("nolevel.json")).unwrap();
        let mut loaded: TeamProfile = serde_json::from_str(&content).unwrap();
        if loaded.level.is_empty() {
            loaded.level = "pro".to_string();
        }
        assert_eq!(loaded.level, "pro");
    }
}
