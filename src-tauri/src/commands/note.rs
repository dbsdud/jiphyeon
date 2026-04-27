use std::fs;

use chrono::Local;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::config::ConfigState;
use crate::editor::{resolve_editor, ResolvedEditor};
use crate::error::AppError;
use crate::models::RenderedNote;
use crate::vault::renderer;

const QUICK_NOTE_FOLDER: &str = "inbox";

#[tauri::command]
pub fn get_note(
    config_state: State<'_, ConfigState>,
    path: String,
) -> Result<RenderedNote, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::NoteNotFound(e.to_string()))?;
    let project = config.active_project().ok_or(AppError::VaultNotConfigured)?;
    let abs_path = project.docs_path.join(&path);
    if !abs_path.exists() {
        return Err(AppError::NoteNotFound(path));
    }

    renderer::render_note(&abs_path, &project.docs_path, &[])
}

#[tauri::command]
pub fn open_in_editor(
    app_handle: tauri::AppHandle,
    config_state: State<'_, ConfigState>,
    path: String,
) -> Result<(), AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::NoteNotFound(e.to_string()))?;
    let project = config.active_project().ok_or(AppError::VaultNotConfigured)?;
    let abs_path = project.docs_path.join(&path);
    if !abs_path.exists() {
        return Err(AppError::NoteNotFound(path));
    }

    match resolve_editor(&config.editor_command, &abs_path) {
        ResolvedEditor::Command { program, args } => {
            std::process::Command::new(&program)
                .args(&args)
                .spawn()
                .map_err(AppError::Io)?;
        }
        ResolvedEditor::Url(url) => {
            app_handle
                .opener()
                .open_url(url, None::<&str>)
                .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub fn create_quick_note(
    config_state: State<'_, ConfigState>,
    title: Option<String>,
    content: String,
    tags: Vec<String>,
    project_id: Option<String>,
) -> Result<String, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::NoteNotFound(e.to_string()))?;
    let project = match project_id.as_deref() {
        Some(id) => config
            .projects
            .iter()
            .find(|p| p.id == id)
            .ok_or(AppError::VaultNotConfigured)?,
        None => config.active_project().ok_or(AppError::VaultNotConfigured)?,
    };
    let today = Local::now().format("%Y-%m-%d").to_string();

    let filename = match &title {
        Some(t) if !t.trim().is_empty() => {
            let s = slug::slugify(t.trim());
            format!("{}-{}.md", today, s)
        }
        _ => {
            let timestamp = Local::now().format("%H%M%S").to_string();
            format!("{}-{}.md", today, timestamp)
        }
    };

    let inbox_dir = project.docs_path.join(QUICK_NOTE_FOLDER);
    fs::create_dir_all(&inbox_dir)?;

    let mut file_path = inbox_dir.join(&filename);
    let mut counter = 1;
    while file_path.exists() {
        let stem = filename.trim_end_matches(".md");
        file_path = inbox_dir.join(format!("{}-{}.md", stem, counter));
        counter += 1;
    }

    let tags_yaml = if tags.is_empty() {
        "tags: []".to_string()
    } else {
        let tag_list: Vec<String> = tags.iter().map(|t| format!("  - {}", t)).collect();
        format!("tags:\n{}", tag_list.join("\n"))
    };

    let note_title = title
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or("Quick Note");

    let note_content = format!(
        "---\ntype: idea\ncreated: {}\nstatus: seedling\n{}\n---\n\n# {}\n\n{}",
        today, tags_yaml, note_title, content
    );

    fs::write(&file_path, &note_content)?;

    let relative_path = file_path
        .strip_prefix(&project.docs_path)
        .unwrap_or(&file_path)
        .to_string_lossy()
        .to_string();

    Ok(relative_path)
}
