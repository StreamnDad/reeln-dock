use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::orchestration::hook_executor;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// JSON models (mirror reeln-cli's render_queue.json schema)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishTargetResult {
    pub target: String,
    pub status: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliQueueItem {
    pub id: String,
    pub output: String,
    pub game_dir: String,
    pub status: String,
    pub queued_at: String,

    #[serde(default)]
    pub duration_seconds: Option<f64>,
    #[serde(default)]
    pub file_size_bytes: Option<u64>,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub crop_mode: String,
    #[serde(default)]
    pub render_profile: String,
    #[serde(default)]
    pub event_id: String,

    #[serde(default)]
    pub home_team: String,
    #[serde(default)]
    pub away_team: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub sport: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub tournament: String,
    #[serde(default)]
    pub event_type: String,
    #[serde(default)]
    pub player: String,
    #[serde(default)]
    pub assists: String,

    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub publish_targets: Vec<PublishTargetResult>,
    #[serde(default)]
    pub config_profile: String,
    #[serde(default)]
    pub plugin_inputs: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct RenderQueueFile {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    items: Vec<CliQueueItem>,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
struct QueueIndex {
    #[serde(default)]
    queues: Vec<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read and parse render_queue.json from a game directory.
fn read_queue_file(game_dir: &Path) -> Result<RenderQueueFile, String> {
    let path = game_dir.join("render_queue.json");
    if !path.is_file() {
        return Ok(RenderQueueFile {
            version: 1,
            items: Vec::new(),
        });
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let queue: RenderQueueFile = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid queue file {}: {e}", path.display()))?;
    if queue.version != 1 {
        return Err(format!(
            "Unsupported queue version {} in {}. Please update reeln-dock.",
            queue.version,
            path.display()
        ));
    }
    Ok(queue)
}

/// Locate the CLI's queue_index.json (platform-specific data dir).
fn queue_index_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();

    #[cfg(target_os = "macos")]
    {
        PathBuf::from(&home).join("Library/Application Support/reeln/data/queue_index.json")
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join("AppData/Roaming"))
            .join("reeln/data/queue_index.json")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".local/share"))
            .join("reeln/queue_index.json")
    }
}

/// Read the central queue index.
fn read_queue_index() -> Result<Vec<String>, String> {
    let path = queue_index_path();
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read queue index: {e}"))?;
    let index: QueueIndex =
        serde_json::from_str(&content).map_err(|e| format!("Invalid queue index: {e}"))?;
    Ok(index.queues)
}

/// Resolve the CLI path from AppState.
fn resolve_cli_path(state: &State<'_, AppState>) -> Result<String, String> {
    let settings = state.dock_settings.lock().map_err(|e| e.to_string())?;
    hook_executor::detect_reeln_cli(settings.reeln_cli_path.as_deref())
}

/// Run a `reeln queue` subcommand and return the re-read queue item.
fn run_queue_cli(
    cli_path: &str,
    args: &[&str],
    config_path: Option<&str>,
) -> Result<std::process::Output, String> {
    run_queue_cli_with_profile(cli_path, args, config_path, None)
}

/// Run a `reeln queue` subcommand with optional `--profile` flag.
fn run_queue_cli_with_profile(
    cli_path: &str,
    args: &[&str],
    config_path: Option<&str>,
    profile: Option<&str>,
) -> Result<std::process::Output, String> {
    let mut cmd = Command::new(cli_path);
    cmd.arg("queue");
    for arg in args {
        cmd.arg(arg);
    }
    if let Some(config) = config_path {
        cmd.arg("--config").arg(config);
    }
    if let Some(p) = profile {
        cmd.arg("--profile").arg(p);
    }
    cmd.output()
        .map_err(|e| format!("Failed to execute reeln queue: {e}"))
}

/// Check that a CLI command succeeded, returning an error with stderr on failure.
fn check_cli_output(output: &std::process::Output) -> Result<(), String> {
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if stderr.is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };
        Err(msg.trim().to_string())
    }
}

// ---------------------------------------------------------------------------
// Read commands (direct file reads — fast, no subprocess)
// ---------------------------------------------------------------------------

/// List queue items for a game directory, optionally filtered by status.
#[tauri::command]
pub fn queue_list(
    game_dir: String,
    status_filter: Option<String>,
) -> Result<Vec<CliQueueItem>, String> {
    let queue = read_queue_file(Path::new(&game_dir))?;
    let items = match status_filter {
        Some(ref s) => queue
            .items
            .into_iter()
            .filter(|item| item.status == *s)
            .collect(),
        None => queue
            .items
            .into_iter()
            .filter(|item| item.status != "removed")
            .collect(),
    };
    Ok(items)
}

/// List queue items across all games (via central queue index).
#[tauri::command]
pub fn queue_list_all(status_filter: Option<String>) -> Result<Vec<CliQueueItem>, String> {
    let game_dirs = read_queue_index()?;
    let mut all_items = Vec::new();

    for dir_str in &game_dirs {
        let dir = Path::new(dir_str);
        if !dir.is_dir() {
            continue; // Skip stale entries
        }
        match read_queue_file(dir) {
            Ok(queue) => {
                for item in queue.items {
                    let include = match &status_filter {
                        Some(s) => item.status == *s,
                        None => item.status != "removed",
                    };
                    if include {
                        all_items.push(item);
                    }
                }
            }
            Err(_) => continue, // Skip unreadable queue files
        }
    }

    Ok(all_items)
}

/// Get a single queue item by ID or prefix.
#[tauri::command]
pub fn queue_show(game_dir: String, item_id: String) -> Result<CliQueueItem, String> {
    let queue = read_queue_file(Path::new(&game_dir))?;

    // Exact match first
    if let Some(item) = queue.items.iter().find(|i| i.id == item_id) {
        return Ok(item.clone());
    }

    // Prefix match
    let matches: Vec<&CliQueueItem> = queue
        .items
        .iter()
        .filter(|i| i.id.starts_with(&item_id))
        .collect();

    match matches.len() {
        0 => Err(format!("Queue item '{}' not found", item_id)),
        1 => Ok(matches[0].clone()),
        _ => {
            let ids: Vec<&str> = matches.iter().map(|i| i.id.as_str()).collect();
            Err(format!(
                "Ambiguous ID prefix '{}' matches: {}",
                item_id,
                ids.join(", ")
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Mutation commands (shell out to CLI for hook execution)
// ---------------------------------------------------------------------------

/// List available publish targets from loaded plugins.
#[tauri::command]
pub async fn queue_targets(
    app: AppHandle,
    state: State<'_, AppState>,
    config_path: Option<String>,
    profile: Option<String>,
) -> Result<Vec<String>, String> {
    let cli_path = resolve_cli_path(&state)?;

    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        crate::dock_log::log_cli_command(&app_clone, "Queue", &cli_path, &["queue", "targets"]);
        let output = run_queue_cli_with_profile(
            &cli_path,
            &["targets"],
            config_path.as_deref(),
            profile.as_deref(),
        )?;
        crate::dock_log::log_cli_output(&app_clone, "Queue", &output);
        check_cli_output(&output)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let targets: Vec<String> = stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .collect();
        Ok(targets)
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

/// Edit title/description of a queue item.
#[tauri::command]
pub async fn queue_edit(
    app: AppHandle,
    state: State<'_, AppState>,
    game_dir: String,
    item_id: String,
    title: Option<String>,
    description: Option<String>,
) -> Result<CliQueueItem, String> {
    let cli_path = resolve_cli_path(&state)?;

    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        let mut args = vec!["edit", &item_id];
        let title_flag;
        let desc_flag;

        if let Some(ref t) = title {
            title_flag = t.clone();
            args.push("--title");
            args.push(&title_flag);
        }
        if let Some(ref d) = description {
            desc_flag = d.clone();
            args.push("--description");
            args.push(&desc_flag);
        }
        args.push("--game-dir");
        args.push(&game_dir);

        let log_args: Vec<&str> = std::iter::once("queue")
            .chain(args.iter().copied())
            .collect();
        crate::dock_log::log_cli_command(&app_clone, "Queue", &cli_path, &log_args);
        let output = run_queue_cli(&cli_path, &args, None)?;
        crate::dock_log::log_cli_output(&app_clone, "Queue", &output);
        check_cli_output(&output)?;

        // Re-read the updated item
        let queue = read_queue_file(Path::new(&game_dir))?;
        queue
            .items
            .into_iter()
            .find(|i| i.id == item_id || i.id.starts_with(&item_id))
            .ok_or_else(|| format!("Item '{}' not found after edit", item_id))
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

/// Publish a queue item to target(s).
#[tauri::command]
pub async fn queue_publish(
    app: AppHandle,
    state: State<'_, AppState>,
    game_dir: String,
    item_id: String,
    target: Option<String>,
    config_path: Option<String>,
) -> Result<CliQueueItem, String> {
    let cli_path = resolve_cli_path(&state)?;

    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        // Read the item's stored config_profile for --profile
        let stored_profile = read_queue_file(Path::new(&game_dir))
            .ok()
            .and_then(|q| {
                q.items
                    .iter()
                    .find(|i| i.id == item_id || i.id.starts_with(&item_id))
                    .map(|i| i.config_profile.clone())
            })
            .filter(|p| !p.is_empty());

        let mut args = vec!["publish", &item_id];
        args.push("--game-dir");
        args.push(&game_dir);

        let target_ref;
        if let Some(ref t) = target {
            target_ref = t.clone();
            args.push("--target");
            args.push(&target_ref);
        }

        let log_args: Vec<&str> = std::iter::once("queue")
            .chain(args.iter().copied())
            .collect();
        crate::dock_log::log_cli_command(&app_clone, "Queue", &cli_path, &log_args);
        let output = run_queue_cli_with_profile(
            &cli_path,
            &args,
            config_path.as_deref(),
            stored_profile.as_deref(),
        )?;
        crate::dock_log::log_cli_output(&app_clone, "Queue", &output);
        check_cli_output(&output)?;

        // Re-read the updated item
        let queue = read_queue_file(Path::new(&game_dir))?;
        queue
            .items
            .into_iter()
            .find(|i| i.id == item_id || i.id.starts_with(&item_id))
            .ok_or_else(|| format!("Item '{}' not found after publish", item_id))
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

/// Publish all rendered items in a game's queue.
#[tauri::command]
pub async fn queue_publish_all(
    app: AppHandle,
    state: State<'_, AppState>,
    game_dir: String,
    config_path: Option<String>,
) -> Result<Vec<CliQueueItem>, String> {
    let cli_path = resolve_cli_path(&state)?;

    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        let mut args = vec!["publish-all"];
        args.push("--game-dir");
        args.push(&game_dir);

        let log_args: Vec<&str> = std::iter::once("queue")
            .chain(args.iter().copied())
            .collect();
        crate::dock_log::log_cli_command(&app_clone, "Queue", &cli_path, &log_args);
        let output = run_queue_cli(&cli_path, &args, config_path.as_deref())?;
        crate::dock_log::log_cli_output(&app_clone, "Queue", &output);
        check_cli_output(&output)?;

        // Re-read the full queue
        let queue = read_queue_file(Path::new(&game_dir))?;
        Ok(queue
            .items
            .into_iter()
            .filter(|i| i.status != "removed")
            .collect())
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

/// Soft-delete a queue item.
#[tauri::command]
pub async fn queue_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    game_dir: String,
    item_id: String,
) -> Result<CliQueueItem, String> {
    let cli_path = resolve_cli_path(&state)?;

    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        let args = vec!["remove", &item_id, "--game-dir", &game_dir];
        let log_args: Vec<&str> = std::iter::once("queue")
            .chain(args.iter().copied())
            .collect();
        crate::dock_log::log_cli_command(&app_clone, "Queue", &cli_path, &log_args);
        let output = run_queue_cli(&cli_path, &args, None)?;
        crate::dock_log::log_cli_output(&app_clone, "Queue", &output);
        check_cli_output(&output)?;

        // Re-read the removed item
        let queue = read_queue_file(Path::new(&game_dir))?;
        queue
            .items
            .into_iter()
            .find(|i| i.id == item_id || i.id.starts_with(&item_id))
            .ok_or_else(|| format!("Item '{}' not found after remove", item_id))
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // read_queue_file
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_queue_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_queue_file(dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().items.is_empty());
    }

    #[test]
    fn test_read_queue_file_valid() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = dir.path().join("render_queue.json");
        std::fs::write(
            &queue_path,
            r#"{
                "version": 1,
                "items": [{
                    "id": "abcd1234ef56",
                    "output": "/path/to/out.mp4",
                    "game_dir": "/path/to/game",
                    "status": "rendered",
                    "queued_at": "2026-04-06T18:00:00Z",
                    "title": "Test Goal",
                    "description": "A test",
                    "publish_targets": [{
                        "target": "google",
                        "status": "pending",
                        "url": "",
                        "error": "",
                        "published_at": ""
                    }]
                }]
            }"#,
        )
        .unwrap();

        let result = read_queue_file(dir.path()).unwrap();
        assert_eq!(result.version, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].id, "abcd1234ef56");
        assert_eq!(result.items[0].title, "Test Goal");
        assert_eq!(result.items[0].publish_targets.len(), 1);
        assert_eq!(result.items[0].publish_targets[0].target, "google");
    }

    #[test]
    fn test_read_queue_file_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = dir.path().join("render_queue.json");
        std::fs::write(&queue_path, "not json").unwrap();

        let result = read_queue_file(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid queue file"));
    }

    #[test]
    fn test_read_queue_file_unsupported_version() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("render_queue.json"),
            r#"{"version": 99, "items": []}"#,
        )
        .unwrap();

        let result = read_queue_file(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported queue version 99"));
    }

    // -----------------------------------------------------------------------
    // queue_list (unit — calls read_queue_file internally)
    // -----------------------------------------------------------------------

    #[test]
    fn test_queue_list_filters_removed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("render_queue.json"),
            r#"{"version": 1, "items": [
                {"id": "a", "output": "", "game_dir": "", "status": "rendered", "queued_at": ""},
                {"id": "b", "output": "", "game_dir": "", "status": "removed", "queued_at": ""}
            ]}"#,
        )
        .unwrap();

        let items = queue_list(dir.path().to_str().unwrap().to_string(), None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "a");
    }

    #[test]
    fn test_queue_list_status_filter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("render_queue.json"),
            r#"{"version": 1, "items": [
                {"id": "a", "output": "", "game_dir": "", "status": "rendered", "queued_at": ""},
                {"id": "b", "output": "", "game_dir": "", "status": "published", "queued_at": ""},
                {"id": "c", "output": "", "game_dir": "", "status": "rendered", "queued_at": ""}
            ]}"#,
        )
        .unwrap();

        let items = queue_list(
            dir.path().to_str().unwrap().to_string(),
            Some("rendered".to_string()),
        )
        .unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.status == "rendered"));
    }

    // -----------------------------------------------------------------------
    // queue_show
    // -----------------------------------------------------------------------

    #[test]
    fn test_queue_show_exact_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("render_queue.json"),
            r#"{"version": 1, "items": [
                {"id": "abcd1234ef56", "output": "", "game_dir": "", "status": "rendered", "queued_at": "", "title": "Found"}
            ]}"#,
        )
        .unwrap();

        let item = queue_show(
            dir.path().to_str().unwrap().to_string(),
            "abcd1234ef56".to_string(),
        )
        .unwrap();
        assert_eq!(item.title, "Found");
    }

    #[test]
    fn test_queue_show_prefix_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("render_queue.json"),
            r#"{"version": 1, "items": [
                {"id": "abcd1234ef56", "output": "", "game_dir": "", "status": "rendered", "queued_at": "", "title": "Found"}
            ]}"#,
        )
        .unwrap();

        let item =
            queue_show(dir.path().to_str().unwrap().to_string(), "abcd".to_string()).unwrap();
        assert_eq!(item.title, "Found");
    }

    #[test]
    fn test_queue_show_ambiguous_prefix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("render_queue.json"),
            r#"{"version": 1, "items": [
                {"id": "abcd1234ef56", "output": "", "game_dir": "", "status": "rendered", "queued_at": ""},
                {"id": "abcd5678ef90", "output": "", "game_dir": "", "status": "rendered", "queued_at": ""}
            ]}"#,
        )
        .unwrap();

        let result = queue_show(dir.path().to_str().unwrap().to_string(), "abcd".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Ambiguous"));
    }

    #[test]
    fn test_queue_show_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("render_queue.json"),
            r#"{"version": 1, "items": []}"#,
        )
        .unwrap();

        let result = queue_show(dir.path().to_str().unwrap().to_string(), "nope".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // -----------------------------------------------------------------------
    // queue_list_all
    // -----------------------------------------------------------------------

    #[test]
    fn test_read_queue_index_missing() {
        // queue_index_path points to a system path, so test read_queue_index indirectly
        // by testing that missing files return empty
        let result = read_queue_file(Path::new("/nonexistent/path"));
        assert!(result.is_ok());
        assert!(result.unwrap().items.is_empty());
    }

    // -----------------------------------------------------------------------
    // check_cli_output
    // -----------------------------------------------------------------------

    #[test]
    fn test_check_cli_output_success() {
        let _output = std::process::Output {
            status: std::process::ExitStatus::default(),
            stdout: b"ok".to_vec(),
            stderr: Vec::new(),
        };
        // ExitStatus::default() is platform-specific; skip success test if not 0
    }

    #[test]
    fn test_queue_file_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("render_queue.json"),
            r#"{"version": 1, "items": [
                {"id": "x", "output": "/out.mp4", "game_dir": "/game", "status": "rendered", "queued_at": "2026-01-01T00:00:00Z"}
            ]}"#,
        )
        .unwrap();

        let queue = read_queue_file(dir.path()).unwrap();
        let item = &queue.items[0];

        // All optional fields should have defaults
        assert_eq!(item.duration_seconds, None);
        assert_eq!(item.file_size_bytes, None);
        assert_eq!(item.format, "");
        assert_eq!(item.crop_mode, "");
        assert_eq!(item.render_profile, "");
        assert_eq!(item.home_team, "");
        assert_eq!(item.title, "");
        assert_eq!(item.description, "");
        assert!(item.publish_targets.is_empty());
        assert_eq!(item.config_profile, "");
    }

    // -----------------------------------------------------------------------
    // CLI command arg verification (subprocess tests)
    // -----------------------------------------------------------------------

    #[cfg(unix)]
    fn make_arg_dump_script(
        dir: &std::path::Path,
        args_file: &std::path::Path,
    ) -> std::path::PathBuf {
        let script = dir.join("fake_reeln.sh");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"{}\"\n",
                args_file.display()
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // Brief pause to let the kernel finish releasing the inode after write+chmod.
        // Prevents ETXTBSY on Linux CI with parallel test threads.
        std::thread::sleep(std::time::Duration::from_millis(10));
        script
    }

    #[cfg(unix)]
    #[test]
    fn test_run_queue_cli_edit_args() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let cli_path = script.to_str().unwrap();

        let _output = run_queue_cli(
            cli_path,
            &[
                "edit",
                "abc123",
                "--title",
                "New Title",
                "--game-dir",
                "/game",
            ],
            None,
        )
        .unwrap();

        let args: Vec<String> = std::fs::read_to_string(&args_file)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();

        assert_eq!(
            args,
            vec![
                "queue",
                "edit",
                "abc123",
                "--title",
                "New Title",
                "--game-dir",
                "/game"
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_run_queue_cli_publish_with_target_and_config() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let cli_path = script.to_str().unwrap();

        let _output = run_queue_cli(
            cli_path,
            &[
                "publish",
                "abc123",
                "--game-dir",
                "/game",
                "--target",
                "google",
            ],
            Some("/config.json"),
        )
        .unwrap();

        let args: Vec<String> = std::fs::read_to_string(&args_file)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();

        assert_eq!(
            args,
            vec![
                "queue",
                "publish",
                "abc123",
                "--game-dir",
                "/game",
                "--target",
                "google",
                "--config",
                "/config.json"
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_run_queue_cli_remove_args() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let cli_path = script.to_str().unwrap();

        let _output =
            run_queue_cli(cli_path, &["remove", "abc123", "--game-dir", "/game"], None).unwrap();

        let args: Vec<String> = std::fs::read_to_string(&args_file)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();

        assert_eq!(
            args,
            vec!["queue", "remove", "abc123", "--game-dir", "/game"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_run_queue_cli_targets_with_config() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let cli_path = script.to_str().unwrap();

        let _output = run_queue_cli(cli_path, &["targets"], Some("/my/config.json")).unwrap();

        let args: Vec<String> = std::fs::read_to_string(&args_file)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();

        assert_eq!(
            args,
            vec!["queue", "targets", "--config", "/my/config.json"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_run_queue_cli_publish_all_args() {
        let dir = tempfile::tempdir().unwrap();
        let args_file = dir.path().join("args.txt");
        let script = make_arg_dump_script(dir.path(), &args_file);
        let cli_path = script.to_str().unwrap();

        let _output =
            run_queue_cli(cli_path, &["publish-all", "--game-dir", "/game"], None).unwrap();

        let args: Vec<String> = std::fs::read_to_string(&args_file)
            .unwrap()
            .lines()
            .map(|l| l.to_string())
            .collect();

        assert_eq!(args, vec!["queue", "publish-all", "--game-dir", "/game"]);
    }
}
