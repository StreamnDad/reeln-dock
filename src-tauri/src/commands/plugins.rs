use tauri::State;

use crate::state::AppState;

/// A config profile discovered from the config directory.
#[derive(serde::Serialize)]
pub struct ConfigProfile {
    /// Display name (e.g. "production-google", "meta-ig-test")
    pub name: String,
    /// Full file path
    pub path: String,
    /// Whether this is the currently active profile
    pub active: bool,
}

/// Plugin info with settings from a specific config profile.
#[derive(serde::Serialize)]
pub struct PluginDetail {
    pub name: String,
    pub enabled: bool,
    pub settings: serde_json::Value,
}

/// A plugin entry from the registry (available for install/add).
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct RegistryPlugin {
    pub name: String,
    pub package: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub homepage: String,
    pub min_reeln_version: String,
    pub author: String,
    pub license: String,
    /// Plugin UI contributions — screens and fields the plugin declares.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_contributions: Option<serde_json::Value>,
}

#[derive(serde::Deserialize)]
struct PluginRegistry {
    #[allow(dead_code)]
    registry_version: u32,
    plugins: Vec<RegistryPlugin>,
}

/// List config profile files in the config directory.
/// Discovers `config.*.json` files plus the active config.
#[tauri::command]
pub fn list_config_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<ConfigProfile>, String> {
    let config_dir = state.effective_config_dir();
    let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
    let active_path = settings.reeln_config_path.clone().unwrap_or_default();

    let mut profiles = Vec::new();

    if !config_dir.is_dir() {
        return Ok(profiles);
    }

    // Canonicalize the active path for reliable comparison
    let active_canonical = if !active_path.is_empty() {
        std::fs::canonicalize(&active_path)
            .unwrap_or_else(|_| std::path::PathBuf::from(&active_path))
    } else {
        std::path::PathBuf::new()
    };

    for entry in std::fs::read_dir(&config_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        if !filename.ends_with(".json") {
            continue;
        }

        // Read file and check if it looks like a reeln config
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let raw: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Must have a "plugins" section or "config_version" to be a config file
        let is_config = raw.get("plugins").is_some() || raw.get("config_version").is_some();
        if !is_config {
            continue;
        }

        let path_canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let is_active = path_canonical == active_canonical;

        let name = derive_profile_name(filename);
        profiles.push(ConfigProfile {
            name,
            path: path.display().to_string(),
            active: is_active,
        });
    }

    profiles.sort_by(|a, b| {
        // Active profile first, then alphabetical
        b.active.cmp(&a.active).then(a.name.cmp(&b.name))
    });

    Ok(profiles)
}

/// List plugins and their settings from a specific config profile.
#[tauri::command]
pub fn list_plugins_for_profile(
    profile_path: String,
) -> Result<Vec<PluginDetail>, String> {
    let config = load_config_file(&profile_path)?;
    let mut plugins = Vec::new();

    // Enabled plugins
    for name in &config.plugins.enabled {
        let settings = config
            .plugins
            .settings
            .get(name)
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        plugins.push(PluginDetail {
            name: name.clone(),
            enabled: true,
            settings,
        });
    }

    // Disabled plugins
    for name in &config.plugins.disabled {
        let settings = config
            .plugins
            .settings
            .get(name)
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        plugins.push(PluginDetail {
            name: name.clone(),
            enabled: false,
            settings,
        });
    }

    // Any plugins with settings but not in either list
    for name in config.plugins.settings.keys() {
        if !config.plugins.enabled.contains(name) && !config.plugins.disabled.contains(name) {
            let settings = config.plugins.settings[name].clone();
            plugins.push(PluginDetail {
                name: name.clone(),
                enabled: false,
                settings,
            });
        }
    }

    plugins.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(plugins)
}

/// Toggle a plugin's enabled/disabled status in a config profile.
#[tauri::command]
pub fn toggle_plugin_in_config(
    profile_path: String,
    plugin_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<PluginDetail>, String> {
    let mut raw = load_raw_config(&profile_path)?;

    let plugins = raw
        .get_mut("plugins")
        .and_then(|p| p.as_object_mut())
        .ok_or("Config missing 'plugins' section")?;

    // Ensure both arrays exist
    if !plugins.contains_key("enabled") {
        plugins.insert("enabled".to_string(), serde_json::json!([]));
    }
    if !plugins.contains_key("disabled") {
        plugins.insert("disabled".to_string(), serde_json::json!([]));
    }

    let name_val = serde_json::Value::String(plugin_name.clone());

    // Check current state by reading (immutable)
    let in_enabled = plugins["enabled"]
        .as_array()
        .map(|a| a.contains(&name_val))
        .unwrap_or(false);
    let in_disabled = plugins["disabled"]
        .as_array()
        .map(|a| a.contains(&name_val))
        .unwrap_or(false);

    if in_enabled {
        // Move from enabled to disabled
        let arr = plugins.get_mut("enabled").unwrap().as_array_mut().unwrap();
        arr.retain(|v| v != &name_val);
        let arr = plugins.get_mut("disabled").unwrap().as_array_mut().unwrap();
        arr.push(name_val);
    } else if in_disabled {
        // Move from disabled to enabled
        let arr = plugins.get_mut("disabled").unwrap().as_array_mut().unwrap();
        arr.retain(|v| v != &name_val);
        let arr = plugins.get_mut("enabled").unwrap().as_array_mut().unwrap();
        arr.push(name_val);
    } else {
        // Not in either list — add to enabled
        let arr = plugins.get_mut("enabled").unwrap().as_array_mut().unwrap();
        arr.push(name_val);
    }

    save_raw_config(&profile_path, &raw)?;

    // Reload into AppState if this is the active config
    reload_if_active(&profile_path, &state)?;

    list_plugins_for_profile(profile_path)
}

/// Update a single plugin's settings in a config profile.
#[tauri::command]
pub fn update_plugin_in_config(
    profile_path: String,
    plugin_name: String,
    settings: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<Vec<PluginDetail>, String> {
    let mut raw = load_raw_config(&profile_path)?;

    let plugins = raw
        .get_mut("plugins")
        .and_then(|p| p.as_object_mut())
        .ok_or("Config missing 'plugins' section")?;

    let plugin_settings = plugins
        .entry("settings")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or("plugins.settings is not an object")?;

    plugin_settings.insert(plugin_name, settings);

    save_raw_config(&profile_path, &raw)?;

    // Reload into AppState if this is the active config
    reload_if_active(&profile_path, &state)?;

    list_plugins_for_profile(profile_path)
}

/// Fetch the plugin registry from the local workspace or a remote URL.
/// Tries workspace-relative path first, then the configured registry URL,
/// then the default GitHub raw URL.
#[tauri::command]
pub fn fetch_plugin_registry(
    state: State<'_, AppState>,
) -> Result<Vec<RegistryPlugin>, String> {
    // 1. Try workspace-relative path (dev mode)
    let workspace_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reeln-cli/registry/plugins.json");
    if workspace_path.is_file() {
        let content = std::fs::read_to_string(&workspace_path).map_err(|e| e.to_string())?;
        let registry: PluginRegistry =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;
        return Ok(registry.plugins);
    }

    // 2. Try config-relative path
    let config_registry = state.effective_config_dir().join("registry/plugins.json");
    if config_registry.is_file() {
        let content = std::fs::read_to_string(&config_registry).map_err(|e| e.to_string())?;
        let registry: PluginRegistry =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;
        return Ok(registry.plugins);
    }

    Err("Plugin registry not found. Place registry/plugins.json in the config directory or workspace.".to_string())
}

/// Add a plugin to a config profile (inserts into the `enabled` array).
/// If the plugin is already present in the config, this is a no-op.
#[tauri::command]
pub fn add_plugin_to_config(
    profile_path: String,
    plugin_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<PluginDetail>, String> {
    let mut raw = load_raw_config(&profile_path)?;

    let plugins = raw
        .get_mut("plugins")
        .and_then(|p| p.as_object_mut())
        .ok_or("Config missing 'plugins' section")?;

    // Ensure both arrays exist
    if !plugins.contains_key("enabled") {
        plugins.insert("enabled".to_string(), serde_json::json!([]));
    }
    if !plugins.contains_key("disabled") {
        plugins.insert("disabled".to_string(), serde_json::json!([]));
    }

    let name_val = serde_json::Value::String(plugin_name.clone());

    // Check if already present
    let in_enabled = plugins["enabled"]
        .as_array()
        .map(|a| a.contains(&name_val))
        .unwrap_or(false);
    let in_disabled = plugins["disabled"]
        .as_array()
        .map(|a| a.contains(&name_val))
        .unwrap_or(false);

    if !in_enabled && !in_disabled {
        // Add to enabled
        let arr = plugins
            .get_mut("enabled")
            .unwrap()
            .as_array_mut()
            .unwrap();
        arr.push(name_val);

        // Create empty settings entry
        let settings = plugins
            .entry("settings")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or("plugins.settings is not an object")?;
        if !settings.contains_key(&plugin_name) {
            settings.insert(plugin_name, serde_json::json!({}));
        }

        save_raw_config(&profile_path, &raw)?;
        reload_if_active(&profile_path, &state)?;
    }

    list_plugins_for_profile(profile_path)
}

/// Remove a plugin from a config profile entirely (from enabled, disabled, and settings).
#[tauri::command]
pub fn remove_plugin_from_config(
    profile_path: String,
    plugin_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<PluginDetail>, String> {
    let mut raw = load_raw_config(&profile_path)?;

    let plugins = raw
        .get_mut("plugins")
        .and_then(|p| p.as_object_mut())
        .ok_or("Config missing 'plugins' section")?;

    let name_val = serde_json::Value::String(plugin_name.clone());

    // Remove from enabled
    if let Some(arr) = plugins.get_mut("enabled").and_then(|v| v.as_array_mut()) {
        arr.retain(|v| v != &name_val);
    }
    // Remove from disabled
    if let Some(arr) = plugins.get_mut("disabled").and_then(|v| v.as_array_mut()) {
        arr.retain(|v| v != &name_val);
    }
    // Remove settings entry
    if let Some(settings) = plugins.get_mut("settings").and_then(|v| v.as_object_mut()) {
        settings.remove(&plugin_name);
    }

    save_raw_config(&profile_path, &raw)?;
    reload_if_active(&profile_path, &state)?;

    list_plugins_for_profile(profile_path)
}

/// Create a new config profile by copying the active config and resetting plugins.
/// The new file is named `config.<profile_name>.json` in the same directory.
#[tauri::command]
pub fn create_config_profile(
    profile_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<ConfigProfile>, String> {
    let config_dir = state.effective_config_dir();
    let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
    let active_path = settings.reeln_config_path.clone().unwrap_or_default();
    drop(settings);

    // Load the active config as a raw JSON base (or create a minimal one)
    let mut raw = if !active_path.is_empty() && std::path::Path::new(&active_path).is_file() {
        load_raw_config(&active_path)?
    } else {
        serde_json::json!({
            "config_version": 1,
            "plugins": { "enabled": [], "disabled": [], "settings": {} }
        })
    };

    // Reset plugins to empty for the new profile
    if let Some(plugins) = raw.get_mut("plugins").and_then(|p| p.as_object_mut()) {
        plugins.insert("enabled".to_string(), serde_json::json!([]));
        plugins.insert("disabled".to_string(), serde_json::json!([]));
        plugins.insert("settings".to_string(), serde_json::json!({}));
    }

    let filename = format!("config.{profile_name}.json");
    let new_path = config_dir.join(&filename);
    if new_path.exists() {
        return Err(format!("Profile '{profile_name}' already exists"));
    }

    save_raw_config(&new_path.display().to_string(), &raw)?;

    // Return updated profiles list
    list_config_profiles(state)
}

/// Get version information for the app and config.
#[tauri::command]
pub fn get_version_info(
    state: State<'_, AppState>,
) -> Result<VersionInfo, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let config_version = config.as_ref().map(|c| c.config_version);

    Ok(VersionInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        config_version,
    })
}

#[derive(serde::Serialize)]
pub struct VersionInfo {
    pub app_version: String,
    pub config_version: Option<u32>,
}

// ── Helpers ─────────────────────────────────────────────────────────

fn derive_profile_name(filename: &str) -> String {
    let stem = filename.trim_end_matches(".json");
    if stem == "config" {
        return "default".to_string();
    }
    // "config.production-google" -> "production-google"
    // "game.v1" -> "game.v1"
    stem.strip_prefix("config.").unwrap_or(stem).to_string()
}

fn load_config_file(path: &str) -> Result<reeln_config::AppConfig, String> {
    let p = std::path::Path::new(path);
    if !p.exists() {
        return Err(format!("Config file not found: {path}"));
    }
    reeln_config::load_config(p, None).map_err(|e| e.to_string())
}

fn load_raw_config(path: &str) -> Result<serde_json::Value, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_raw_config(path: &str, value: &serde_json::Value) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    // Atomic write: write to temp, then rename
    let tmp = format!("{path}.tmp");
    std::fs::write(&tmp, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get the enforce_hooks setting from a config profile.
#[tauri::command]
pub fn get_enforce_hooks(profile_path: String) -> Result<bool, String> {
    let raw = load_raw_config(&profile_path)?;
    let enforce = raw
        .get("plugins")
        .and_then(|p| p.get("enforce_hooks"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    Ok(enforce)
}

/// Set the enforce_hooks setting in a config profile.
#[tauri::command]
pub fn set_enforce_hooks(
    profile_path: String,
    enforce: bool,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let mut raw = load_raw_config(&profile_path)?;

    let plugins = raw
        .get_mut("plugins")
        .and_then(|p| p.as_object_mut())
        .ok_or("Config missing 'plugins' section")?;

    plugins.insert("enforce_hooks".to_string(), serde_json::Value::Bool(enforce));

    save_raw_config(&profile_path, &raw)?;
    reload_if_active(&profile_path, &state)?;

    Ok(enforce)
}

fn reload_if_active(profile_path: &str, state: &AppState) -> Result<(), String> {
    let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
    let active = settings.reeln_config_path.as_deref().unwrap_or("");
    if active == profile_path {
        drop(settings);
        let config = load_config_file(profile_path)?;
        let mut locked = state.config.lock().map_err(|e| e.to_string())?;
        *locked = Some(config);
    }
    Ok(())
}

