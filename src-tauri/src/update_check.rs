use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Wry};

const DOCK_REPO: &str = "StreamnDad/reeln-dock";
const CLI_REPO: &str = "StreamnDad/reeln-cli";
const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60; // 24 hours
const STARTUP_DELAY_SECS: u64 = 10; // wait before first check

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub name: String,          // "reeln-dock" or "reeln-cli"
    pub current: String,       // installed version
    pub latest: String,        // latest release version
    pub release_notes: String, // body from GitHub release
    pub release_url: String,   // HTML URL to the release page
    pub published_at: String,  // release date
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheckResult {
    pub updates: Vec<UpdateInfo>,
}

/// Fetch the latest release from a GitHub repo. Returns (tag, body, html_url, published_at).
fn fetch_latest_release(repo: &str) -> Result<(String, String, String, String), String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let response = ureq::get(&url)
        .set("User-Agent", "reeln-dock")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("Failed to fetch {repo} releases: {e}"))?;

    let json: serde_json::Value = response
        .into_json()
        .map_err(|e| format!("Failed to parse {repo} release JSON: {e}"))?;

    let tag = json["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    let body = json["body"].as_str().unwrap_or("").to_string();
    let html_url = json["html_url"].as_str().unwrap_or("").to_string();
    let published = json["published_at"].as_str().unwrap_or("").to_string();

    if tag.is_empty() {
        return Err(format!("No releases found for {repo}"));
    }

    Ok((tag, body, html_url, published))
}

/// Parse a version string, stripping leading 'v' if present.
fn parse_version(s: &str) -> Option<semver::Version> {
    semver::Version::parse(s.trim_start_matches('v')).ok()
}

/// An installed plugin with its current version.
pub struct InstalledPlugin {
    pub name: String,
    pub version: String,
}

/// Check for updates and return any available.
pub fn check(
    dock_version: &str,
    cli_version: Option<&str>,
    plugins: &[InstalledPlugin],
) -> UpdateCheckResult {
    let mut updates = Vec::new();

    // Check reeln-dock
    if let Ok((latest_tag, body, url, published)) = fetch_latest_release(DOCK_REPO) {
        let current = parse_version(dock_version);
        let latest = parse_version(&latest_tag);
        if let (Some(c), Some(l)) = (current, latest)
            && l > c
        {
            updates.push(UpdateInfo {
                name: "reeln-dock".to_string(),
                current: dock_version.to_string(),
                latest: latest_tag,
                release_notes: body,
                release_url: url,
                published_at: published,
            });
        }
    }

    // Check reeln-cli
    if let Some(cli_ver) = cli_version
        && let Ok((latest_tag, body, url, published)) = fetch_latest_release(CLI_REPO)
    {
        let current = parse_version(cli_ver);
        let latest = parse_version(&latest_tag);
        if let (Some(c), Some(l)) = (current, latest)
            && l > c
        {
            updates.push(UpdateInfo {
                name: "reeln-cli".to_string(),
                current: cli_ver.to_string(),
                latest: latest_tag,
                release_notes: body,
                release_url: url,
                published_at: published,
            });
        }
    }

    // Check installed plugins (repos follow StreamnDad/reeln-plugin-{name})
    for plugin in plugins {
        let repo = format!("StreamnDad/reeln-plugin-{}", plugin.name);
        if let Ok((latest_tag, body, url, published)) = fetch_latest_release(&repo) {
            let current = parse_version(&plugin.version);
            let latest = parse_version(&latest_tag);
            if let (Some(c), Some(l)) = (current, latest)
                && l > c
            {
                updates.push(UpdateInfo {
                    name: format!("reeln-plugin-{}", plugin.name),
                    current: plugin.version.clone(),
                    latest: latest_tag,
                    release_notes: body,
                    release_url: url,
                    published_at: published,
                });
            }
        }
    }

    UpdateCheckResult { updates }
}

/// Spawn background update check on app startup.
pub fn spawn_startup_check(app: &AppHandle<Wry>) {
    let app_handle = app.clone();
    let dock_version = env!("CARGO_PKG_VERSION").to_string();

    std::thread::spawn(move || {
        // Wait a bit before checking — let the app load first
        std::thread::sleep(std::time::Duration::from_secs(STARTUP_DELAY_SECS));

        // Check if we should skip (checked recently)
        if let Some(state) = app_handle.try_state::<crate::state::AppState>() {
            let settings = state.dock_settings.lock().ok();
            if let Some(ref s) = settings
                && let Some(last) = s.last_update_check
            {
                let now = chrono::Utc::now().timestamp() as u64;
                if now.saturating_sub(last) < CHECK_INTERVAL_SECS {
                    return; // checked recently
                }
            }
        }

        // Get CLI version and installed plugins
        let (cli_version, plugins) = detect_cli_and_plugins(&app_handle);

        let result = check(&dock_version, cli_version.as_deref(), &plugins);

        // Update last check timestamp
        if let Some(state) = app_handle.try_state::<crate::state::AppState>()
            && let Ok(mut settings) = state.dock_settings.lock()
        {
            settings.last_update_check = Some(chrono::Utc::now().timestamp() as u64);
            let _ = settings.save(&state.app_data_dir);
        }

        if !result.updates.is_empty() {
            let _ = app_handle.emit("update:available", &result);
        }
    });
}

/// Detect CLI version and installed plugins from `reeln --version` output.
pub fn detect_cli_and_plugins(app: &AppHandle<Wry>) -> (Option<String>, Vec<InstalledPlugin>) {
    let state = match app.try_state::<crate::state::AppState>() {
        Some(s) => s,
        None => return (None, Vec::new()),
    };
    let settings = match state.dock_settings.lock() {
        Ok(s) => s,
        Err(_) => return (None, Vec::new()),
    };
    let cli_path = match crate::orchestration::hook_executor::detect_reeln_cli(
        settings.reeln_cli_path.as_deref(),
    ) {
        Ok(p) => p,
        Err(_) => return (None, Vec::new()),
    };
    drop(settings);

    let output = match std::process::Command::new(&cli_path)
        .arg("--version")
        .output()
    {
        Ok(o) => o,
        Err(_) => return (None, Vec::new()),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut cli_version = None;
    let mut plugins = Vec::new();
    let mut in_plugins = false;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("reeln ") && !trimmed.contains("native") {
            let ver = trimmed
                .strip_prefix("reeln ")
                .unwrap_or(trimmed)
                .trim_start_matches('v');
            if semver::Version::parse(ver).is_ok() {
                cli_version = Some(ver.to_string());
            }
        } else if trimmed == "plugins:" {
            in_plugins = true;
        } else if in_plugins && !trimmed.is_empty() {
            let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
            if parts.len() == 2 {
                plugins.push(InstalledPlugin {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                });
            }
        }
    }

    (cli_version, plugins)
}
