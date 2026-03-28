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
