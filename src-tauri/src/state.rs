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
/// Mirrors all fields in `render_ops::RenderOverrides` so every CLI-supported
/// override can be preset in dock settings.
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pad_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom_frames: Option<u32>,
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
    /// PID of an in-progress auth subprocess (for cancel support).
    pub auth_child_pid: Arc<Mutex<Option<u32>>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;
    use reeln_sport::SportRegistry;
    use std::sync::Mutex;

    #[test]
    fn dock_settings_default_fields() {
        let settings = DockSettings::default();
        assert!(settings.reeln_config_path.is_none());
        assert!(settings.plugin_profiles.is_empty());
        assert!(settings.reeln_cli_path.is_none());
        // display defaults
        assert!(settings.display.show_logos);
        assert!(settings.display.sections_expanded.games);
        assert!(settings.display.sections_expanded.teams);
        assert!(settings.display.sections_expanded.tournaments);
        // rendering defaults
        assert!(settings.rendering.iteration_mappings.is_empty());
        assert!(settings.rendering.default_profile.is_none());
        assert!(settings.rendering.default_plugin_profile.is_none());
        assert!(settings.rendering.default_render_mode.is_none());
        assert!(!settings.rendering.concat_by_default);
        assert!(settings.rendering.plugin_field_defaults.is_empty());
    }

    #[test]
    fn dock_settings_file_path() {
        let dir = Path::new("/tmp/test-dock");
        let path = DockSettings::file_path(dir);
        assert_eq!(path, dir.join("dock-settings.json"));
    }

    #[test]
    fn dock_settings_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = DockSettings::default();
        settings.reeln_config_path = Some("/some/path/config.json".to_string());
        settings.reeln_cli_path = Some("/usr/local/bin/reeln".to_string());
        settings.rendering.default_profile = Some("hd".to_string());
        settings.rendering.concat_by_default = true;
        settings.rendering.overrides.scale = Some(0.5);
        settings.rendering.overrides.speed = Some(1.5);
        settings.rendering.overrides.smart = Some(true);
        settings.rendering.overrides.crop_mode = Some("center".to_string());
        settings.display.show_logos = false;

        let saved_path = settings.save(dir.path()).unwrap();
        assert!(saved_path.exists());

        let loaded = DockSettings::load(dir.path()).expect("should load saved settings");
        assert_eq!(loaded.reeln_config_path.as_deref(), Some("/some/path/config.json"));
        assert_eq!(loaded.reeln_cli_path.as_deref(), Some("/usr/local/bin/reeln"));
        assert_eq!(loaded.rendering.default_profile.as_deref(), Some("hd"));
        assert!(loaded.rendering.concat_by_default);
        assert_eq!(loaded.rendering.overrides.scale, Some(0.5));
        assert_eq!(loaded.rendering.overrides.speed, Some(1.5));
        assert_eq!(loaded.rendering.overrides.smart, Some(true));
        assert_eq!(loaded.rendering.overrides.crop_mode.as_deref(), Some("center"));
        assert!(!loaded.display.show_logos);
    }

    #[test]
    fn dock_settings_load_missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = DockSettings::load(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn dock_settings_save_creates_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        let settings = DockSettings::default();

        let saved_path = settings.save(&nested).unwrap();
        assert!(saved_path.exists());
        assert!(nested.is_dir());
    }

    #[test]
    fn effective_config_dir_with_reeln_config_path() {
        let dir = tempfile::tempdir().unwrap();
        // Create the config directory so parent.is_dir() returns true
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("config.json");

        let mut settings = DockSettings::default();
        settings.reeln_config_path = Some(config_file.to_string_lossy().to_string());

        let state = AppState {
            config: Mutex::new(None),
            sport_registry: Mutex::new(SportRegistry::default()),
            dock_settings: Mutex::new(settings),
            app_data_dir: dir.path().to_path_buf(),
            media_backend: test_utils::mock_backend(),
            auth_child_pid: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };

        assert_eq!(state.effective_config_dir(), config_dir);
    }

    #[test]
    fn effective_config_dir_without_reeln_config_path() {
        let dir = tempfile::tempdir().unwrap();
        let settings = DockSettings::default();

        let state = AppState {
            config: Mutex::new(None),
            sport_registry: Mutex::new(SportRegistry::default()),
            dock_settings: Mutex::new(settings),
            app_data_dir: dir.path().to_path_buf(),
            media_backend: test_utils::mock_backend(),
            auth_child_pid: std::sync::Arc::new(std::sync::Mutex::new(None)),
        };

        // Without a config path set, should fall back to reeln_config::config_dir()
        assert_eq!(state.effective_config_dir(), reeln_config::config_dir());
    }

    #[test]
    fn rendering_defaults_default_fields() {
        let rd = RenderingDefaults::default();
        assert!(rd.iteration_mappings.is_empty());
        assert!(rd.default_profile.is_none());
        assert!(rd.default_plugin_profile.is_none());
        assert!(rd.default_render_mode.is_none());
        assert!(!rd.concat_by_default);
        assert!(rd.plugin_field_defaults.is_empty());
    }

    #[test]
    fn render_override_defaults_all_none() {
        let rod = RenderOverrideDefaults::default();
        assert!(rod.crop_mode.is_none());
        assert!(rod.scale.is_none());
        assert!(rod.speed.is_none());
        assert!(rod.smart.is_none());
        assert!(rod.anchor_x.is_none());
        assert!(rod.anchor_y.is_none());
        assert!(rod.pad_color.is_none());
        assert!(rod.zoom_frames.is_none());
    }
}
