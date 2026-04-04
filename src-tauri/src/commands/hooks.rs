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
    let (cli_version, plugins) = parse_version_output(&stdout);

    Ok(CliVersionInfo {
        cli_version,
        cli_path,
        plugins,
    })
}

/// Parse `reeln --version` output into version string and plugin list.
/// Extracted for testability.
fn parse_version_output(stdout: &str) -> (String, Vec<CliPluginInfo>) {
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

    (cli_version, plugins)
}

/// Install a plugin via `reeln plugins install <name>`.
#[derive(serde::Serialize)]
pub struct PluginInstallResult {
    pub success: bool,
    pub output: String,
}

#[tauri::command]
pub async fn install_plugin_via_cli(
    state: State<'_, AppState>,
    plugin_name: String,
) -> Result<PluginInstallResult, String> {
    let cli_path_override = {
        let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
        settings.reeln_cli_path.clone()
    };

    let reeln_path = hook_executor::detect_reeln_cli(cli_path_override.as_deref())?;

    tokio::task::spawn_blocking(move || {
        let output = Command::new(&reeln_path)
            .arg("plugins")
            .arg("install")
            .arg(&plugin_name)
            .output()
            .map_err(|e| format!("Failed to run reeln plugins install: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(PluginInstallResult {
                success: true,
                output: stdout,
            })
        } else {
            Ok(PluginInstallResult {
                success: false,
                output: if stderr.is_empty() { stdout } else { stderr },
            })
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // -----------------------------------------------------------------------
    // parse_version_output
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_version_typical_output() {
        let stdout = "reeln 0.8.2\n\
                       plugins:\n\
                       youtube 0.3.1\n\
                       openai 0.2.0\n";

        let (version, plugins) = parse_version_output(stdout);

        assert_eq!(version, "0.8.2");
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name, "youtube");
        assert_eq!(plugins[0].version, "0.3.1");
        assert_eq!(plugins[1].name, "openai");
        assert_eq!(plugins[1].version, "0.2.0");
    }

    #[test]
    fn test_parse_version_no_plugins() {
        let stdout = "reeln 0.8.2\nplugins:\n";
        let (version, plugins) = parse_version_output(stdout);
        assert_eq!(version, "0.8.2");
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_parse_version_no_plugins_section() {
        let stdout = "reeln 0.8.2\n";
        let (version, plugins) = parse_version_output(stdout);
        assert_eq!(version, "0.8.2");
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_parse_version_empty_output() {
        let (version, plugins) = parse_version_output("");
        assert_eq!(version, "");
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_parse_version_skips_native_line() {
        let stdout = "reeln native 0.1.0\nreeln 0.8.2\nplugins:\n";
        let (version, _) = parse_version_output(stdout);
        assert_eq!(version, "0.8.2");
    }

    #[test]
    fn test_parse_version_plugin_without_version() {
        let stdout = "reeln 0.8.2\nplugins:\nlocal-only\n";
        let (_, plugins) = parse_version_output(stdout);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "local-only");
        assert_eq!(plugins[0].version, "");
    }

    // -----------------------------------------------------------------------
    // install_plugin_via_cli — subprocess arg verification
    // -----------------------------------------------------------------------

    fn make_arg_dump_script(dir: &std::path::Path, args_file: &std::path::Path) -> std::path::PathBuf {
        let script = dir.join("fake_reeln.sh");
        let mut f = std::fs::File::create(&script).unwrap();
        writeln!(f, "#!/bin/sh").unwrap();
        writeln!(
            f,
            "printf '%s\\n' \"$@\" > \"{}\"",
            args_file.display()
        )
        .unwrap();
        writeln!(f, "echo 'installed successfully'").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    }

    #[test]
    fn test_install_plugin_subprocess_args() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);

        // Simulate what install_plugin_via_cli does inside spawn_blocking
        let reeln_path = script.to_str().unwrap().to_string();
        let plugin_name = "reeln-youtube".to_string();

        let output = std::process::Command::new(&reeln_path)
            .arg("plugins")
            .arg("install")
            .arg(&plugin_name)
            .output()
            .unwrap();

        assert!(output.status.success());

        let args: Vec<String> = std::fs::read_to_string(&args_file)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();

        assert_eq!(args, vec!["plugins", "install", "reeln-youtube"]);
    }

    #[test]
    fn test_install_plugin_failure_returns_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("fail_install.sh");
        {
            let mut f = std::fs::File::create(&script).unwrap();
            writeln!(f, "#!/bin/sh").unwrap();
            writeln!(f, "echo 'error: plugin not found' >&2").unwrap();
            writeln!(f, "exit 1").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }

        let output = std::process::Command::new(script.to_str().unwrap())
            .arg("plugins")
            .arg("install")
            .arg("nonexistent")
            .output()
            .unwrap();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        assert!(!output.status.success());
        let result_output = if stderr.is_empty() { stdout } else { stderr };
        assert!(result_output.contains("plugin not found"));
    }
}
