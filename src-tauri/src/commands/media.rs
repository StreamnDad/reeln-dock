use std::path::Path;

use tauri::State;

use crate::models::MediaInfoResponse;
use crate::orchestration::render_ops;
use crate::state::AppState;

#[tauri::command]
pub fn probe_clip(path: String) -> Result<MediaInfoResponse, String> {
    let p = Path::new(&path);
    let info = reeln_media::probe::probe(p).map_err(|e| e.to_string())?;
    Ok(info.into())
}

#[tauri::command]
pub fn open_in_finder(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(p)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(p)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(p.parent().unwrap_or(p))
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {path}"));
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(p)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(p)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(p)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Generate an MP4 proxy for non-web video formats (MKV, AVI, TS, FLV).
/// Returns the original path if already web-playable, or the cached/new proxy path.
#[tauri::command]
pub async fn prepare_preview_proxy(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let backend = state.media_backend.clone();
    let proxy_dir = state.app_data_dir.join("proxies");
    let input = std::path::PathBuf::from(&path);

    tokio::task::spawn_blocking(move || {
        let result = render_ops::prepare_preview_proxy(&backend, &input, &proxy_dir)?;
        Ok(result.display().to_string())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Get proxy cache stats: total size and file count.
#[derive(serde::Serialize)]
pub struct ProxyCacheStats {
    pub file_count: u32,
    pub total_bytes: u64,
}

#[tauri::command]
pub fn get_proxy_cache_stats(state: State<'_, AppState>) -> ProxyCacheStats {
    let proxy_dir = state.app_data_dir.join("proxies");
    let mut file_count = 0u32;
    let mut total_bytes = 0u64;

    if let Ok(entries) = std::fs::read_dir(&proxy_dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    file_count += 1;
                    total_bytes += meta.len();
                }
            }
        }
    }

    ProxyCacheStats { file_count, total_bytes }
}

/// Clear all proxy cache files.
#[tauri::command]
pub fn clear_proxy_cache(state: State<'_, AppState>) -> Result<u32, String> {
    let proxy_dir = state.app_data_dir.join("proxies");
    let mut removed = 0u32;

    if let Ok(entries) = std::fs::read_dir(&proxy_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let _ = std::fs::remove_file(&path);
                removed += 1;
            }
        }
    }

    Ok(removed)
}

#[tauri::command]
pub fn file_exists(path: String) -> bool {
    Path::new(&path).exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── file_exists ──────────────────────────────────────────────────

    #[test]
    fn file_exists_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();

        assert!(file_exists(file_path.to_str().unwrap().to_string()));
    }

    #[test]
    fn file_exists_nonexistent_path() {
        assert!(!file_exists("/nonexistent/path/to/file.txt".to_string()));
    }

    #[test]
    fn file_exists_existing_directory() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(file_exists(tmp.path().to_str().unwrap().to_string()));
    }

    // ── open_in_finder ───────────────────────────────────────────────

    #[test]
    fn open_in_finder_nonexistent_path_errors() {
        let result = open_in_finder("/nonexistent/path/to/file.txt".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    // ── open_file ────────────────────────────────────────────────────

    #[test]
    fn open_file_nonexistent_path_errors() {
        let result = open_file("/nonexistent/path/to/file.txt".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }
}
