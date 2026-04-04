use std::path::Path;

use crate::models::MediaInfoResponse;

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
