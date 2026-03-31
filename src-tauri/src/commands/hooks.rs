use std::process::Command;

use tauri::State;

use crate::orchestration::hook_executor::{self, HookExecutionResult};
use crate::state::AppState;

#[tauri::command]
pub fn detect_reeln_cli(state: State<'_, AppState>) -> Result<String, String> {
    let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
    hook_executor::detect_reeln_cli(settings.reeln_cli_path.as_deref())
}

/// Installed plugin info from `reeln --version` output.
#[derive(serde::Serialize)]
pub struct CliPluginInfo {
    pub name: String,
    pub version: String,
}

/// Full CLI version info.
#[derive(serde::Serialize)]
pub struct CliVersionInfo {
    pub cli_version: String,
    pub cli_path: String,
    pub plugins: Vec<CliPluginInfo>,
}

/// Detect CLI and query its version + installed plugins.
#[tauri::command]
pub fn get_cli_version(state: State<'_, AppState>) -> Result<CliVersionInfo, String> {
    let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
    let cli_path = hook_executor::detect_reeln_cli(settings.reeln_cli_path.as_deref())?;

    let output = Command::new(&cli_path)
        .arg("--version")
        .output()
        .map_err(|e| format!("Failed to run reeln --version: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut cli_version = String::new();
    let mut plugins = Vec::new();
    let mut in_plugins = false;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("reeln ") && !trimmed.contains("native") {
            cli_version = trimmed.strip_prefix("reeln ").unwrap_or(trimmed).to_string();
        } else if trimmed == "plugins:" {
            in_plugins = true;
        } else if in_plugins && !trimmed.is_empty() {
            let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
            plugins.push(CliPluginInfo {
                name: parts[0].to_string(),
                version: parts.get(1).unwrap_or(&"").to_string(),
            });
        }
    }

    Ok(CliVersionInfo {
        cli_version,
        cli_path,
        plugins,
    })
}

#[tauri::command]
pub async fn execute_plugin_hook(
    state: State<'_, AppState>,
    hook: String,
    context_data: serde_json::Value,
    shared: serde_json::Value,
    config_path: Option<String>,
) -> Result<HookExecutionResult, String> {
    let (cli_path_override, effective_config) = {
        let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
        (
            settings.reeln_cli_path.clone(),
            // Use explicit config_path from frontend, fall back to DockSettings default
            config_path.or_else(|| settings.reeln_config_path.clone()),
        )
    };

    let reeln_path = hook_executor::detect_reeln_cli(cli_path_override.as_deref())?;

    // Run the CLI subprocess on a blocking thread
    tokio::task::spawn_blocking(move || {
        hook_executor::execute_hook(
            &reeln_path,
            &hook,
            &context_data,
            &shared,
            effective_config.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}
