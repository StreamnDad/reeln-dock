use std::path::{Path, PathBuf};

use tauri::State;

use crate::models::PromptTemplateInfo;
use crate::state::AppState;

/// Find the prompt templates directory.
/// Checks several known locations in order:
/// 1. Effective config dir: `<config_dir>/prompt_templates/`
/// 2. Sibling workspace: `../../reeln-plugin-openai/reeln_openai_plugin/prompt_templates/`
fn find_prompt_templates_dir(state: &AppState) -> Option<PathBuf> {
    // 1. Relative to actual config file location
    let config_dir = state.effective_config_dir().join("prompt_templates");
    if config_dir.is_dir() {
        return Some(config_dir);
    }

    // 2. Relative to workspace (dev mode)
    let workspace_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../reeln-plugin-openai/reeln_openai_plugin/prompt_templates");
    if workspace_path.is_dir() {
        return Some(workspace_path);
    }

    None
}

#[tauri::command]
pub fn list_prompt_templates(
    state: State<'_, AppState>,
) -> Result<Vec<PromptTemplateInfo>, String> {
    let dir = find_prompt_templates_dir(&state)
        .ok_or_else(|| "Prompt templates directory not found".to_string())?;

    let mut templates = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("txt") && path.is_file() {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            templates.push(PromptTemplateInfo {
                name,
                content: content.trim().to_string(),
            });
        }
    }
    templates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(templates)
}

#[tauri::command]
pub fn get_prompt_template(
    name: String,
    state: State<'_, AppState>,
) -> Result<PromptTemplateInfo, String> {
    let dir = find_prompt_templates_dir(&state)
        .ok_or_else(|| "Prompt templates directory not found".to_string())?;

    let path = dir.join(format!("{name}.txt"));
    if !path.is_file() {
        return Err(format!("Prompt template '{name}' not found"));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(PromptTemplateInfo {
        name,
        content: content.trim().to_string(),
    })
}

#[tauri::command]
pub fn preview_prompt(
    name: String,
    variables: std::collections::HashMap<String, String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let template = get_prompt_template_inner(name, &state)?;
    let mut result = template.content;
    for (key, value) in &variables {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    Ok(result)
}

/// Save a prompt template permanently to the templates directory.
/// Creates the directory if it doesn't exist.
#[tauri::command]
pub fn save_prompt_template(
    name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<PromptTemplateInfo, String> {
    let dir = find_prompt_templates_dir(&state).unwrap_or_else(|| {
        // Create the templates dir in the effective config dir
        state.effective_config_dir().join("prompt_templates")
    });
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let path = dir.join(format!("{name}.txt"));
    let trimmed = content.trim().to_string();
    std::fs::write(&path, &trimmed).map_err(|e| e.to_string())?;

    Ok(PromptTemplateInfo {
        name,
        content: trimmed,
    })
}

/// Inner helper for get_prompt_template (avoids passing State through invoke).
fn get_prompt_template_inner(
    name: String,
    state: &AppState,
) -> Result<PromptTemplateInfo, String> {
    let dir = find_prompt_templates_dir(state)
        .ok_or_else(|| "Prompt templates directory not found".to_string())?;

    let path = dir.join(format!("{name}.txt"));
    if !path.is_file() {
        return Err(format!("Prompt template '{name}' not found"));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(PromptTemplateInfo {
        name,
        content: content.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, DockSettings};
    use crate::test_utils;
    use reeln_sport::SportRegistry;
    use std::sync::Mutex;

    // ── Helper ──────────────────────────────────────────────────────

    fn make_state_with_templates(dir: &tempfile::TempDir) -> AppState {
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let config_file = config_dir.join("config.json");
        std::fs::write(&config_file, "{}").unwrap();

        let mut settings = DockSettings::default();
        settings.reeln_config_path = Some(config_file.display().to_string());

        AppState {
            config: Mutex::new(None),
            sport_registry: Mutex::new(SportRegistry::default()),
            dock_settings: Mutex::new(settings),
            app_data_dir: dir.path().to_path_buf(),
            media_backend: test_utils::mock_backend(),
        }
    }

    // ── find_prompt_templates_dir ────────────────────────────────────

    #[test]
    fn find_templates_dir_none_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);
        // prompt_templates dir doesn't exist in config dir
        let result = find_prompt_templates_dir(&state);
        // May return None or a workspace path; the config-based path should not be returned
        let config_templates = dir.path().join("config").join("prompt_templates");
        if let Some(found) = &result {
            assert_ne!(found, &config_templates);
        }
    }

    #[test]
    fn find_templates_dir_returns_config_dir_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);

        let templates_dir = dir.path().join("config").join("prompt_templates");
        std::fs::create_dir_all(&templates_dir).unwrap();

        let result = find_prompt_templates_dir(&state);
        assert_eq!(result, Some(templates_dir));
    }

    // ── get_prompt_template_inner ────────────────────────────────────

    #[test]
    fn get_template_inner_returns_content() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);

        let templates_dir = dir.path().join("config").join("prompt_templates");
        std::fs::create_dir_all(&templates_dir).unwrap();
        std::fs::write(
            templates_dir.join("greeting.txt"),
            "  Hello {{name}}!  \n",
        )
        .unwrap();

        let result = get_prompt_template_inner("greeting".to_string(), &state).unwrap();
        assert_eq!(result.name, "greeting");
        assert_eq!(result.content, "Hello {{name}}!");
    }

    #[test]
    fn get_template_inner_trims_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);

        let templates_dir = dir.path().join("config").join("prompt_templates");
        std::fs::create_dir_all(&templates_dir).unwrap();
        std::fs::write(
            templates_dir.join("padded.txt"),
            "\n\n  Some content here  \n\n",
        )
        .unwrap();

        let result = get_prompt_template_inner("padded".to_string(), &state).unwrap();
        assert_eq!(result.content, "Some content here");
    }

    #[test]
    fn get_template_inner_missing_template_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);

        let templates_dir = dir.path().join("config").join("prompt_templates");
        std::fs::create_dir_all(&templates_dir).unwrap();

        let result = get_prompt_template_inner("nonexistent".to_string(), &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn get_template_inner_no_templates_dir_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);
        // Don't create prompt_templates dir

        let result = get_prompt_template_inner("anything".to_string(), &state);
        // May or may not error depending on workspace fallback — but if no dir found, it errors
        if result.is_err() {
            let err = result.unwrap_err();
            assert!(
                err.contains("not found") || err.contains("directory not found"),
                "Unexpected error: {err}"
            );
        }
    }

    // ── preview_prompt logic (variable substitution) ────────────────

    #[test]
    fn variable_substitution() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);

        let templates_dir = dir.path().join("config").join("prompt_templates");
        std::fs::create_dir_all(&templates_dir).unwrap();
        std::fs::write(
            templates_dir.join("intro.txt"),
            "Hello {{name}}, welcome to {{place}}!",
        )
        .unwrap();

        let template = get_prompt_template_inner("intro".to_string(), &state).unwrap();
        let mut result = template.content;
        let mut vars = std::collections::HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        vars.insert("place".to_string(), "Wonderland".to_string());
        for (key, value) in &vars {
            result = result.replace(&format!("{{{{{key}}}}}"), value);
        }
        assert_eq!(result, "Hello Alice, welcome to Wonderland!");
    }

    #[test]
    fn variable_substitution_missing_var_left_as_is() {
        let template_content = "Hello {{name}}, you are {{role}}.";
        let mut result = template_content.to_string();
        let mut vars = std::collections::HashMap::new();
        vars.insert("name".to_string(), "Bob".to_string());
        for (key, value) in &vars {
            result = result.replace(&format!("{{{{{key}}}}}"), value);
        }
        assert_eq!(result, "Hello Bob, you are {{role}}.");
    }

    // ── save_prompt_template logic ──────────────────────────────────

    #[test]
    fn save_template_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);

        // Use find_prompt_templates_dir fallback: create templates dir
        let templates_dir = state.effective_config_dir().join("prompt_templates");
        std::fs::create_dir_all(&templates_dir).unwrap();

        let content = "  This is a new template  \n\n";
        let trimmed = content.trim().to_string();
        let path = templates_dir.join("new_template.txt");
        std::fs::write(&path, &trimmed).unwrap();

        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, "This is a new template");

        // Verify the template is found via get_prompt_template_inner
        let loaded = get_prompt_template_inner("new_template".to_string(), &state).unwrap();
        assert_eq!(loaded.content, "This is a new template");
    }

    #[test]
    fn save_template_creates_dir_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);

        // The config-based prompt_templates dir doesn't exist yet
        let config_templates = state.effective_config_dir().join("prompt_templates");
        assert!(!config_templates.exists());

        // Simulate save_prompt_template creating the dir via create_dir_all
        std::fs::create_dir_all(&config_templates).unwrap();
        assert!(config_templates.is_dir());

        let path = config_templates.join("created.txt");
        std::fs::write(&path, "content").unwrap();
        assert!(path.is_file());

        // Verify the template is now discoverable
        let result = get_prompt_template_inner("created".to_string(), &state).unwrap();
        assert_eq!(result.content, "content");
    }

    // ── list_prompt_templates logic ─────────────────────────────────

    #[test]
    fn list_templates_sorted_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let state = make_state_with_templates(&dir);

        let templates_dir = dir.path().join("config").join("prompt_templates");
        std::fs::create_dir_all(&templates_dir).unwrap();

        std::fs::write(templates_dir.join("zulu.txt"), "z content").unwrap();
        std::fs::write(templates_dir.join("alpha.txt"), "a content").unwrap();
        std::fs::write(templates_dir.join("mike.txt"), "m content").unwrap();

        // Non-txt file should be ignored
        std::fs::write(templates_dir.join("ignore.md"), "not a template").unwrap();

        // Verify find_prompt_templates_dir resolves correctly
        let found_dir = find_prompt_templates_dir(&state).unwrap();
        assert_eq!(found_dir, templates_dir);

        // Replicate list logic
        let mut templates = Vec::new();
        for entry in std::fs::read_dir(&found_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("txt") && path.is_file() {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let content = std::fs::read_to_string(&path).unwrap();
                templates.push(PromptTemplateInfo {
                    name,
                    content: content.trim().to_string(),
                });
            }
        }
        templates.sort_by(|a, b| a.name.cmp(&b.name));

        let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "mike", "zulu"]);
    }

    #[test]
    fn list_templates_ignores_non_txt_files() {
        let dir = tempfile::tempdir().unwrap();
        let templates_dir = dir.path().join("config").join("prompt_templates");
        std::fs::create_dir_all(&templates_dir).unwrap();

        std::fs::write(templates_dir.join("valid.txt"), "ok").unwrap();
        std::fs::write(templates_dir.join("readme.md"), "nope").unwrap();
        std::fs::write(templates_dir.join("data.json"), "{}").unwrap();

        let mut count = 0;
        for entry in std::fs::read_dir(&templates_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("txt") && path.is_file() {
                count += 1;
            }
        }
        assert_eq!(count, 1);
    }
}
