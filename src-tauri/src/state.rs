use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use reeln_config::AppConfig;
use reeln_media::MediaBackend;
use reeln_sport::SportRegistry;
use serde::{Deserialize, Serialize};

use crate::models::DisplayPreferences;

/// A named plugin configuration profile (e.g. "google-production", "meta-ig-test").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginProfile {
    /// Which plugins are enabled in this profile.
    #[serde(default)]
    pub enabled: Vec<String>,
    /// Per-plugin settings overrides for this profile.
    #[serde(default)]
    pub settings: std::collections::HashMap<String, serde_json::Value>,
}

/// Default overrides for render parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderOverrideDefaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smart: Option<bool>,
}

/// Rendering default preferences, persisted in dock settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenderingDefaults {
    /// Override iteration mappings: event_type → [profile_names].
    #[serde(default)]
    pub iteration_mappings: std::collections::HashMap<String, Vec<String>>,
    /// Default render profile name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    /// Default plugin configuration profile name (e.g. "default", "production").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_plugin_profile: Option<String>,
    /// Default render mode: "short" (crop/scale) or "apply" (full-frame).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_render_mode: Option<String>,
    /// Whether to concatenate multi-format renders by default.
    #[serde(default)]
    pub concat_by_default: bool,
    /// Default render overrides (crop, scale, speed, smart).
    #[serde(default)]
    pub overrides: RenderOverrideDefaults,
    /// Default values for plugin-contributed fields.
    #[serde(default)]
    pub plugin_field_defaults: std::collections::HashMap<String, serde_json::Value>,
}

/// Dock-specific settings, stored separately from the reeln config.
/// Lives in Tauri's app data dir (e.g. ~/Library/Application Support/dad.streamn.reeln-dock/).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockSettings {
    /// Path to the reeln config file (read-only reference).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reeln_config_path: Option<String>,
    /// Named plugin configuration profiles.
    #[serde(default)]
    pub plugin_profiles: std::collections::HashMap<String, PluginProfile>,
    /// Display preferences (logos, UI toggles).
    #[serde(default)]
    pub display: DisplayPreferences,
    /// Explicit path to the `reeln` CLI binary (overrides PATH discovery).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reeln_cli_path: Option<String>,
    /// Rendering default preferences (iteration mappings, default profile, etc.).
    #[serde(default)]
    pub rendering: RenderingDefaults,
}

impl Default for DockSettings {
    fn default() -> Self {
        Self {
            reeln_config_path: None,
            plugin_profiles: std::collections::HashMap::new(),
            display: DisplayPreferences::default(),
            reeln_cli_path: None,
            rendering: RenderingDefaults::default(),
        }
    }
}

impl DockSettings {
    pub fn file_path(app_data_dir: &Path) -> PathBuf {
        app_data_dir.join("dock-settings.json")
    }

    pub fn load(app_data_dir: &Path) -> Option<Self> {
        let path = Self::file_path(app_data_dir);
        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, app_data_dir: &Path) -> Result<PathBuf, String> {
        let path = Self::file_path(app_data_dir);
        std::fs::create_dir_all(app_data_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

pub struct AppState {
    /// The reeln config (read-only, loaded from the path in DockSettings).
    pub config: Mutex<Option<AppConfig>>,
    pub sport_registry: Mutex<SportRegistry>,
    pub dock_settings: Mutex<DockSettings>,
    pub app_data_dir: PathBuf,
    /// Native media backend (LibavBackend) — stateless, Send + Sync.
    pub media_backend: Arc<dyn MediaBackend>,
}

impl AppState {
    /// Return the effective config directory based on the loaded config path.
    ///
    /// If `reeln_config_path` is set (e.g. `~/.config/reeln/config/config.json`),
    /// returns its parent directory. Otherwise falls back to `reeln_config::config_dir()`.
    pub fn effective_config_dir(&self) -> PathBuf {
        if let Ok(settings) = self.dock_settings.lock() {
            if let Some(ref config_path) = settings.reeln_config_path {
                let p = Path::new(config_path);
                if let Some(parent) = p.parent() {
                    if parent.is_dir() {
                        return parent.to_path_buf();
                    }
                }
            }
        }
        reeln_config::config_dir()
    }
}
