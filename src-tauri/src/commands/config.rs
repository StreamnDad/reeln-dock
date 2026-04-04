use std::path::Path;

use tauri::State;

use crate::state::{AppState, DockSettings};

/// Load dock settings + the reeln config they point to (if any).
#[tauri::command]
pub fn load_dock_settings(
    state: State<'_, AppState>,
) -> Result<DockSettingsWithConfig, String> {
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

    Ok(DockSettingsWithConfig {
        settings,
        config,
    })
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
    let mut raw: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| e.to_string())?;

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
}

/// Save render queue to disk (app data dir).
#[tauri::command]
pub fn save_render_queue(
    queue_json: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = state.app_data_dir.join("render-queue.json");
    std::fs::write(&path, &queue_json).map_err(|e| e.to_string())
}

/// Load render queue from disk (app data dir). Returns empty array if not found.
#[tauri::command]
pub fn load_render_queue(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = state.app_data_dir.join("render-queue.json");
    if path.is_file() {
        std::fs::read_to_string(&path).map_err(|e| e.to_string())
    } else {
        Ok("[]".to_string())
    }
}
