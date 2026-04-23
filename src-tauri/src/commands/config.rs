use std::path::Path;

use tauri::State;

use crate::state::{AppState, DockSettings};

/// Load dock settings + the reeln config they point to (if any).
#[tauri::command]
pub fn load_dock_settings(state: State<'_, AppState>) -> Result<DockSettingsWithConfig, String> {
    let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;

    let config = if let Some(ref path) = settings.reeln_config_path {
        match load_reeln_config(path) {
            Ok(c) => {
                let mut locked = state.config.lock().map_err(|e| e.to_string())?;
                *locked = Some(c.clone());
                Some(c)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    Ok(DockSettingsWithConfig {
        settings: settings.clone(),
        config,
    })
}

/// Save dock settings and (re)load the reeln config they point to.
#[tauri::command]
pub fn save_dock_settings(
    settings: DockSettings,
    state: State<'_, AppState>,
) -> Result<DockSettingsWithConfig, String> {
    settings.save(&state.app_data_dir)?;

    let config = if let Some(ref path) = settings.reeln_config_path {
        match load_reeln_config(path) {
            Ok(c) => {
                let mut locked = state.config.lock().map_err(|e| e.to_string())?;
                *locked = Some(c.clone());
                Some(c)
            }
            Err(_) => None,
        }
    } else {
        None
    };

    let mut locked = state.dock_settings.lock().map_err(|e| e.to_string())?;
    *locked = settings.clone();

    Ok(DockSettingsWithConfig { settings, config })
}

/// Load reeln config from an explicit file path, or scan a directory for config files.
#[tauri::command]
pub fn load_config_from_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<LoadedConfig, String> {
    let p = Path::new(&path);

    let config_path = if p.is_file() {
        p.to_path_buf()
    } else if p.is_dir() {
        let candidates = ["reeln.json", ".reeln.json", "config.json"];
        candidates
            .iter()
            .map(|name| p.join(name))
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| {
                format!(
                    "No config file found in {}. Looked for: {}",
                    path,
                    candidates.join(", ")
                )
            })?
    } else {
        return Err(format!("Path does not exist: {path}"));
    };

    let config = load_reeln_config(&config_path.display().to_string())?;
    let resolved_path = config_path.display().to_string();

    let mut locked = state.config.lock().map_err(|e| e.to_string())?;
    *locked = Some(config.clone());

    Ok(LoadedConfig {
        config,
        path: resolved_path,
    })
}

/// Get the default reeln config path (XDG).
#[tauri::command]
pub fn get_config_path(profile: Option<String>) -> String {
    let path = reeln_config::resolve_config_path(None, profile.as_deref());
    path.display().to_string()
}

/// Input format for structured event types from the frontend.
#[derive(serde::Deserialize)]
pub struct EventTypeInput {
    pub name: String,
    #[serde(default)]
    pub team_specific: bool,
}

/// Save event types to the active config file.
#[tauri::command]
pub fn save_event_types(
    event_types: Vec<EventTypeInput>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
    let config_path = settings
        .reeln_config_path
        .clone()
        .ok_or("No config path set")?;
    drop(settings);

    let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let mut raw: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    if let Some(obj) = raw.as_object_mut() {
        if event_types.is_empty() {
            obj.remove("event_types");
        } else {
            let arr: Vec<serde_json::Value> = event_types
                .iter()
                .map(|et| {
                    if et.team_specific {
                        serde_json::json!({"name": et.name, "team_specific": true})
                    } else {
                        serde_json::Value::String(et.name.clone())
                    }
                })
                .collect();
            obj.insert("event_types".to_string(), serde_json::Value::Array(arr));
        }
    }

    let json = serde_json::to_string_pretty(&raw).map_err(|e| e.to_string())?;
    let tmp = format!("{config_path}.tmp");
    std::fs::write(&tmp, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &config_path).map_err(|e| e.to_string())?;

    let config = load_reeln_config(&config_path)?;
    let mut locked = state.config.lock().map_err(|e| e.to_string())?;
    *locked = Some(config);

    Ok(())
}

// ── Helpers ─────────────────────────────────────────────────────────

fn load_reeln_config(path: &str) -> Result<reeln_config::AppConfig, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("Config file not found: {path}"));
    }
    reeln_config::load_config(p, None).map_err(|e| e.to_string())
}

// ── Response types ──────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct DockSettingsWithConfig {
    pub settings: DockSettings,
    pub config: Option<reeln_config::AppConfig>,
}

#[derive(serde::Serialize)]
pub struct LoadedConfig {
    pub config: reeln_config::AppConfig,
    pub path: String,
}

// ── Render profile CRUD ────────────────────────────────────────────

/// Helper: read the raw config JSON and return parsed Value + config path.
fn read_raw_config(state: &AppState) -> Result<(serde_json::Value, String), String> {
    let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
    let config_path = settings
        .reeln_config_path
        .clone()
        .ok_or("No config path set")?;
    drop(settings);

    let content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    let raw: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok((raw, config_path))
}

/// Helper: write raw JSON to config (atomic tmp+rename) and reload into AppState.
fn write_raw_config(
    raw: &serde_json::Value,
    config_path: &str,
    state: &AppState,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(raw).map_err(|e| e.to_string())?;
    let tmp = format!("{config_path}.tmp");
    std::fs::write(&tmp, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, config_path).map_err(|e| e.to_string())?;

    let config = load_reeln_config(config_path)?;
    let mut locked = state.config.lock().map_err(|e| e.to_string())?;
    *locked = Some(config);
    Ok(())
}

/// Save (create or update) a render profile in the active config file.
#[tauri::command]
pub fn save_render_profile(
    profile_key: String,
    profile: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if profile_key.is_empty() {
        return Err("Profile key cannot be empty".to_string());
    }

    let (mut raw, config_path) = read_raw_config(&state)?;

    if let Some(obj) = raw.as_object_mut() {
        let profiles = obj
            .entry("render_profiles")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(profiles_obj) = profiles.as_object_mut() {
            profiles_obj.insert(profile_key, profile);
        } else {
            return Err("render_profiles is not an object in config".to_string());
        }
    }

    write_raw_config(&raw, &config_path, &state)
}

/// Delete a render profile from the active config file.
#[tauri::command]
pub fn delete_render_profile(
    profile_key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (mut raw, config_path) = read_raw_config(&state)?;

    if let Some(obj) = raw.as_object_mut() {
        if let Some(profiles) = obj.get_mut("render_profiles") {
            if let Some(profiles_obj) = profiles.as_object_mut()
                && profiles_obj.remove(&profile_key).is_none()
            {
                return Err(format!("Profile '{profile_key}' not found"));
            }
        } else {
            return Err(format!("Profile '{profile_key}' not found"));
        }
    }

    write_raw_config(&raw, &config_path, &state)
}

/// Rename a render profile key in the active config file (atomic: copy value + delete old).
#[tauri::command]
pub fn rename_render_profile(
    old_key: String,
    new_key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if new_key.is_empty() {
        return Err("New profile key cannot be empty".to_string());
    }
    if old_key == new_key {
        return Ok(());
    }

    let (mut raw, config_path) = read_raw_config(&state)?;

    if let Some(obj) = raw.as_object_mut() {
        if let Some(profiles) = obj.get_mut("render_profiles") {
            if let Some(profiles_obj) = profiles.as_object_mut() {
                if profiles_obj.contains_key(&new_key) {
                    return Err(format!("Profile '{new_key}' already exists"));
                }
                let value = profiles_obj
                    .remove(&old_key)
                    .ok_or_else(|| format!("Profile '{old_key}' not found"))?;
                profiles_obj.insert(new_key, value);
            } else {
                return Err("render_profiles is not an object in config".to_string());
            }
        } else {
            return Err(format!("Profile '{old_key}' not found"));
        }
    }

    write_raw_config(&raw, &config_path, &state)
}

// ── Render queue persistence ────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── get_config_path ────────────────────────────────────────────

    #[test]
    fn get_config_path_none_profile() {
        let path = get_config_path(None);
        // Should return a path ending with "config.json" (the default, no profile infix)
        assert!(path.ends_with("config.json"), "got: {path}");
        // The filename must be exactly "config.json", not "config.<profile>.json"
        let filename = std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(filename, "config.json");
    }

    #[test]
    fn get_config_path_with_profile() {
        let path = get_config_path(Some("production".to_string()));
        assert!(
            path.ends_with("config.production.json"),
            "expected path ending with config.production.json, got: {path}"
        );
    }

    // ── load_reeln_config ──────────────────────────────────────────

    #[test]
    fn load_reeln_config_missing_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let result = load_reeln_config(&path.display().to_string());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("not found"),
            "error should mention 'not found', got: {err}"
        );
    }

    #[test]
    fn load_reeln_config_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        crate::test_utils::create_test_config(&path);

        let config = load_reeln_config(&path.display().to_string()).unwrap();
        assert_eq!(config.config_version, 1);
    }

    // ── EventTypeInput serde ───────────────────────────────────────

    #[test]
    fn event_type_input_with_team_specific_true() {
        let json = serde_json::json!({"name": "Goal", "team_specific": true});
        let input: EventTypeInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.name, "Goal");
        assert!(input.team_specific);
    }

    #[test]
    fn event_type_input_default_team_specific_is_false() {
        let json = serde_json::json!({"name": "Penalty"});
        let input: EventTypeInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.name, "Penalty");
        assert!(!input.team_specific);
    }

    // ── read_raw_config / write_raw_config helpers ─────────────────

    fn make_test_app_state(config_path: &str, app_data_dir: &std::path::Path) -> AppState {
        use std::sync::{Arc, Mutex};
        AppState {
            config: Mutex::new(None),
            sport_registry: Mutex::new(reeln_sport::SportRegistry::default()),
            dock_settings: Mutex::new(DockSettings {
                reeln_config_path: Some(config_path.to_string()),
                ..Default::default()
            }),
            app_data_dir: app_data_dir.to_path_buf(),
            media_backend: crate::test_utils::mock_backend(),
            auth_child_pid: Arc::new(Mutex::new(None)),
        }
    }

    #[test]
    fn read_raw_config_no_config_path_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState {
            config: std::sync::Mutex::new(None),
            sport_registry: std::sync::Mutex::new(reeln_sport::SportRegistry::default()),
            dock_settings: std::sync::Mutex::new(DockSettings::default()),
            app_data_dir: dir.path().to_path_buf(),
            media_backend: crate::test_utils::mock_backend(),
            auth_child_pid: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };
        let result = read_raw_config(&state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No config path set"));
    }

    #[test]
    fn read_raw_config_reads_json() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        crate::test_utils::create_test_config(&config_path);
        let state = make_test_app_state(&config_path.display().to_string(), dir.path());

        let (raw, path) = read_raw_config(&state).unwrap();
        assert!(raw.is_object());
        assert_eq!(path, config_path.display().to_string());
    }

    #[test]
    fn write_raw_config_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        crate::test_utils::create_test_config(&config_path);
        let state = make_test_app_state(&config_path.display().to_string(), dir.path());

        let raw = serde_json::json!({"config_version": 1, "sport": "hockey"});
        write_raw_config(&raw, &config_path.display().to_string(), &state).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["sport"], "hockey");
        // tmp file should be cleaned up
        assert!(!dir.path().join("config.json.tmp").exists());
    }

    #[test]
    fn write_raw_config_reloads_into_app_state() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        crate::test_utils::create_test_config(&config_path);
        let state = make_test_app_state(&config_path.display().to_string(), dir.path());

        assert!(state.config.lock().unwrap().is_none());
        let raw = serde_json::json!({"config_version": 1});
        write_raw_config(&raw, &config_path.display().to_string(), &state).unwrap();
        assert!(state.config.lock().unwrap().is_some());
    }

    // ── save_render_profile logic (tested via helpers) ─────────────

    #[test]
    fn save_profile_creates_render_profiles_key_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        // Write a config with no render_profiles key
        std::fs::write(&config_path, r#"{"config_version":1}"#).unwrap();
        let state = make_test_app_state(&config_path.display().to_string(), dir.path());

        let (mut raw, path) = read_raw_config(&state).unwrap();
        let profile = serde_json::json!({"name": "tiktok", "width": 1080, "height": 1920});

        if let Some(obj) = raw.as_object_mut() {
            let profiles = obj
                .entry("render_profiles")
                .or_insert_with(|| serde_json::json!({}));
            profiles
                .as_object_mut()
                .unwrap()
                .insert("tiktok".to_string(), profile);
        }
        write_raw_config(&raw, &path, &state).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["render_profiles"]["tiktok"]["width"], 1080);
        assert_eq!(parsed["render_profiles"]["tiktok"]["height"], 1920);
    }

    #[test]
    fn save_profile_updates_existing_profile() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{"config_version":1,"render_profiles":{"tiktok":{"width":720,"height":1280}}}"#,
        )
        .unwrap();
        let state = make_test_app_state(&config_path.display().to_string(), dir.path());

        let (mut raw, path) = read_raw_config(&state).unwrap();
        let updated = serde_json::json!({"width": 1080, "height": 1920});
        raw["render_profiles"]["tiktok"] = updated;
        write_raw_config(&raw, &path, &state).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["render_profiles"]["tiktok"]["width"], 1080);
    }

    #[test]
    fn delete_profile_removes_key() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(&config_path, r#"{"config_version":1,"render_profiles":{"tiktok":{"width":1080},"youtube":{"width":1920}}}"#).unwrap();
        let state = make_test_app_state(&config_path.display().to_string(), dir.path());

        let (mut raw, path) = read_raw_config(&state).unwrap();
        raw["render_profiles"]
            .as_object_mut()
            .unwrap()
            .remove("tiktok");
        write_raw_config(&raw, &path, &state).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["render_profiles"].get("tiktok").is_none());
        assert!(parsed["render_profiles"].get("youtube").is_some());
    }

    #[test]
    fn rename_profile_moves_value() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{"config_version":1,"render_profiles":{"old":{"width":1080}}}"#,
        )
        .unwrap();
        let state = make_test_app_state(&config_path.display().to_string(), dir.path());

        let (mut raw, path) = read_raw_config(&state).unwrap();
        let profiles = raw["render_profiles"].as_object_mut().unwrap();
        let value = profiles.remove("old").unwrap();
        profiles.insert("new".to_string(), value);
        write_raw_config(&raw, &path, &state).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed["render_profiles"].get("old").is_none());
        assert_eq!(parsed["render_profiles"]["new"]["width"], 1080);
    }

    #[test]
    fn save_profile_preserves_unknown_fields() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{"config_version":1,"custom_field":"preserved","render_profiles":{}}"#,
        )
        .unwrap();
        let state = make_test_app_state(&config_path.display().to_string(), dir.path());

        let (mut raw, path) = read_raw_config(&state).unwrap();
        raw["render_profiles"]
            .as_object_mut()
            .unwrap()
            .insert("new".to_string(), serde_json::json!({"width": 1080}));
        write_raw_config(&raw, &path, &state).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed["custom_field"], "preserved");
        assert_eq!(parsed["render_profiles"]["new"]["width"], 1080);
    }
}

/// Save render stage (pre-render items) to disk (app data dir).
#[tauri::command]
pub fn save_render_stage(stage_json: String, state: State<'_, AppState>) -> Result<(), String> {
    let path = state.app_data_dir.join("render-stage.json");
    std::fs::write(&path, &stage_json).map_err(|e| e.to_string())
}

/// Load render stage from disk (app data dir). Returns empty array if not found.
/// Migrates from old render-queue.json if render-stage.json doesn't exist.
#[tauri::command]
pub fn load_render_stage(state: State<'_, AppState>) -> Result<String, String> {
    let stage_path = state.app_data_dir.join("render-stage.json");
    if stage_path.is_file() {
        return std::fs::read_to_string(&stage_path).map_err(|e| e.to_string());
    }
    // Migration: check for old render-queue.json
    let old_path = state.app_data_dir.join("render-queue.json");
    if old_path.is_file() {
        let content = std::fs::read_to_string(&old_path).map_err(|e| e.to_string())?;
        // Write to new location and remove old file
        std::fs::write(&stage_path, &content).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&old_path);
        return Ok(content);
    }
    Ok("[]".to_string())
}
