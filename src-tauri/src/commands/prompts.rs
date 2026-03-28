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
