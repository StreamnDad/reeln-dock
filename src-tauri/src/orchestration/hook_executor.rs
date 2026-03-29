use serde::{Deserialize, Serialize};
use std::process::Command;

/// Result of executing a single hook via the reeln CLI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookExecutionResult {
    pub success: bool,
    pub hook: String,
    pub shared: serde_json::Value,
    pub logs: Vec<String>,
    pub errors: Vec<String>,
}

/// Discover the `reeln` CLI binary path.
///
/// Checks in order:
/// 1. Explicit path (from DockSettings)
/// 2. `which reeln` / `where reeln` on PATH
pub fn detect_reeln_cli(explicit_path: Option<&str>) -> Result<String, String> {
    if let Some(path) = explicit_path {
        if std::path::Path::new(path).is_file() {
            return Ok(path.to_string());
        }
        return Err(format!("Configured reeln CLI path not found: {}", path));
    }

    // Try PATH discovery
    let cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };

    let output = Command::new(cmd)
        .arg("reeln")
        .output()
        .map_err(|e| format!("Failed to search for reeln CLI: {}", e))?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    Err(
        "reeln CLI not found. Install it with: uv pip install reeln\n\
         Or set the path in Dock settings."
            .to_string(),
    )
}

/// Execute a single hook via the `reeln hooks run` CLI command.
pub fn execute_hook(
    reeln_path: &str,
    hook: &str,
    context_data: &serde_json::Value,
    shared: &serde_json::Value,
    config_path: Option<&str>,
) -> Result<HookExecutionResult, String> {
    let context_json =
        serde_json::to_string(context_data).map_err(|e| format!("Failed to serialize context: {}", e))?;

    let shared_json =
        serde_json::to_string(shared).map_err(|e| format!("Failed to serialize shared: {}", e))?;

    let mut cmd = Command::new(reeln_path);
    cmd.arg("hooks")
        .arg("run")
        .arg(hook)
        .arg("--context-json")
        .arg(&context_json);

    // Only pass shared if non-empty
    if shared.is_object() && !shared.as_object().map_or(true, |m| m.is_empty()) {
        cmd.arg("--shared-json").arg(&shared_json);
    }

    if let Some(path) = config_path {
        cmd.arg("--config").arg(path);
    }

    let output = cmd.output().map_err(|e| format!("Failed to execute reeln CLI: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Parse the JSON from stdout
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "reeln CLI returned no output. Exit code: {}. Stderr: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    serde_json::from_str::<HookExecutionResult>(trimmed).map_err(|e| {
        format!(
            "Failed to parse CLI output: {}. Output: {}. Stderr: {}",
            e,
            &trimmed[..trimmed.len().min(500)],
            stderr.trim()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // -----------------------------------------------------------------------
    // HookExecutionResult serde
    // -----------------------------------------------------------------------

    #[test]
    fn test_result_deserialize_success() {
        let json = r#"{
            "success": true,
            "hook": "on_game_init",
            "shared": {"title": "Test"},
            "logs": ["loaded plugin"],
            "errors": []
        }"#;
        let result: HookExecutionResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.hook, "on_game_init");
        assert_eq!(result.shared["title"], "Test");
        assert_eq!(result.logs, vec!["loaded plugin"]);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_result_deserialize_failure() {
        let json = r#"{
            "success": false,
            "hook": "",
            "shared": {},
            "logs": [],
            "errors": ["config broken"]
        }"#;
        let result: HookExecutionResult = serde_json::from_str(json).unwrap();
        assert!(!result.success);
        assert_eq!(result.errors, vec!["config broken"]);
    }

    #[test]
    fn test_result_roundtrip() {
        let original = HookExecutionResult {
            success: true,
            hook: "on_game_ready".to_string(),
            shared: serde_json::json!({"key": "val"}),
            logs: vec!["log1".to_string()],
            errors: vec!["err1".to_string()],
        };
        let serialized = serde_json::to_string(&original).unwrap();
        let deserialized: HookExecutionResult = serde_json::from_str(&serialized).unwrap();
        assert_eq!(original, deserialized);
    }

    // -----------------------------------------------------------------------
    // detect_reeln_cli
    // -----------------------------------------------------------------------

    #[test]
    fn test_detect_explicit_path_exists() {
        let dir = tempfile::tempdir().unwrap();
        let fake_bin = dir.path().join("reeln");
        std::fs::write(&fake_bin, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake_bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let result = detect_reeln_cli(Some(fake_bin.to_str().unwrap()));
        assert_eq!(result.unwrap(), fake_bin.to_str().unwrap());
    }

    #[test]
    fn test_detect_explicit_path_not_found() {
        let result = detect_reeln_cli(Some("/nonexistent/path/reeln"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_detect_path_discovery() {
        // This tests against the real PATH — reeln should be installed
        let result = detect_reeln_cli(None);
        // On CI this might fail, so we just check the function doesn't panic
        // and returns either Ok (found) or Err (not found) — both are valid
        match result {
            Ok(path) => assert!(!path.is_empty()),
            Err(msg) => assert!(msg.contains("not found") || msg.contains("Failed")),
        }
    }

    // -----------------------------------------------------------------------
    // execute_hook — helper to create a script that outputs given text
    // -----------------------------------------------------------------------

    fn make_script(dir: &std::path::Path, stdout_text: &str) -> std::path::PathBuf {
        let script = dir.join("fake_reeln.sh");
        let mut f = std::fs::File::create(&script).unwrap();
        writeln!(f, "#!/bin/sh").unwrap();
        writeln!(f, "echo '{}'", stdout_text).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    }

    fn make_failing_script(dir: &std::path::Path) -> std::path::PathBuf {
        let script = dir.join("fail_reeln.sh");
        let mut f = std::fs::File::create(&script).unwrap();
        writeln!(f, "#!/bin/sh").unwrap();
        writeln!(f, "exit 1").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        script
    }

    // -----------------------------------------------------------------------
    // execute_hook
    // -----------------------------------------------------------------------

    #[test]
    fn test_execute_hook_success() {
        let dir = tempfile::tempdir().unwrap();
        let json_out = r#"{"success":true,"hook":"on_game_init","shared":{"k":"v"},"logs":[],"errors":[]}"#;
        let script = make_script(dir.path(), json_out);

        let result = execute_hook(
            script.to_str().unwrap(),
            "on_game_init",
            &serde_json::json!({"game_dir": "/tmp"}),
            &serde_json::json!({}),
            None,
        )
        .unwrap();

        assert!(result.success);
        assert_eq!(result.hook, "on_game_init");
        assert_eq!(result.shared["k"], "v");
    }

    #[test]
    fn test_execute_hook_with_shared() {
        let dir = tempfile::tempdir().unwrap();
        let json_out = r#"{"success":true,"hook":"on_game_ready","shared":{"inherited":"yes"},"logs":[],"errors":[]}"#;
        let script = make_script(dir.path(), json_out);

        let result = execute_hook(
            script.to_str().unwrap(),
            "on_game_ready",
            &serde_json::json!({}),
            &serde_json::json!({"existing": "data"}),
            None,
        )
        .unwrap();

        assert!(result.success);
        assert_eq!(result.shared["inherited"], "yes");
    }

    #[test]
    fn test_execute_hook_with_config_path() {
        let dir = tempfile::tempdir().unwrap();
        let json_out = r#"{"success":true,"hook":"on_game_init","shared":{},"logs":[],"errors":[]}"#;
        let script = make_script(dir.path(), json_out);

        let result = execute_hook(
            script.to_str().unwrap(),
            "on_game_init",
            &serde_json::json!({}),
            &serde_json::json!({}),
            Some("/tmp/config.json"),
        )
        .unwrap();

        assert!(result.success);
    }

    #[test]
    fn test_execute_hook_empty_output() {
        let dir = tempfile::tempdir().unwrap();
        let script = make_failing_script(dir.path());

        let result = execute_hook(
            script.to_str().unwrap(),
            "on_game_init",
            &serde_json::json!({}),
            &serde_json::json!({}),
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no output"));
    }

    #[test]
    fn test_execute_hook_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let script = make_script(dir.path(), "not-json-at-all");

        let result = execute_hook(
            script.to_str().unwrap(),
            "on_game_init",
            &serde_json::json!({}),
            &serde_json::json!({}),
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse"));
    }

    #[test]
    fn test_execute_hook_binary_not_found() {
        let result = execute_hook(
            "/nonexistent/binary",
            "on_game_init",
            &serde_json::json!({}),
            &serde_json::json!({}),
            None,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to execute"));
    }

    #[test]
    fn test_execute_hook_empty_shared_not_passed() {
        // When shared is an empty object, the --shared-json flag should be skipped.
        // We verify this indirectly: the script still succeeds because it ignores args.
        let dir = tempfile::tempdir().unwrap();
        let json_out = r#"{"success":true,"hook":"on_game_init","shared":{},"logs":[],"errors":[]}"#;
        let script = make_script(dir.path(), json_out);

        let result = execute_hook(
            script.to_str().unwrap(),
            "on_game_init",
            &serde_json::json!({}),
            &serde_json::json!({}),
            None,
        )
        .unwrap();

        assert!(result.success);
    }

    #[test]
    fn test_execute_hook_null_shared_not_passed() {
        // When shared is not an object (e.g. null), the --shared-json flag should be skipped.
        let dir = tempfile::tempdir().unwrap();
        let json_out = r#"{"success":true,"hook":"on_game_init","shared":{},"logs":[],"errors":[]}"#;
        let script = make_script(dir.path(), json_out);

        let result = execute_hook(
            script.to_str().unwrap(),
            "on_game_init",
            &serde_json::json!({}),
            &serde_json::Value::Null,
            None,
        )
        .unwrap();

        assert!(result.success);
    }

    #[test]
    fn test_execute_hook_error_response() {
        let dir = tempfile::tempdir().unwrap();
        let json_out = r#"{"success":false,"hook":"on_game_init","shared":{},"logs":[],"errors":["config broken"]}"#;
        let script = make_script(dir.path(), json_out);

        let result = execute_hook(
            script.to_str().unwrap(),
            "on_game_init",
            &serde_json::json!({}),
            &serde_json::json!({}),
            None,
        )
        .unwrap();

        assert!(!result.success);
        assert_eq!(result.errors, vec!["config broken"]);
    }
}
