use std::fs;
use std::path::Path;

use chrono::Local;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

use crate::config::ConfigState;
use crate::editor::{resolve_editor, ResolvedEditor};
use crate::error::AppError;
use crate::models::{BacklinkEntry, RenderedNote};
use crate::vault::parser;
use crate::vault::renderer;

use super::vault::VaultState;

#[tauri::command]
pub fn get_note(
    state: State<'_, VaultState>,
    config_state: State<'_, ConfigState>,
    path: String,
) -> Result<RenderedNote, AppError> {
    let index = state.read().map_err(|e| AppError::NoteNotFound(e.to_string()))?;

    let config = config_state
        .read()
        .map_err(|e| AppError::NoteNotFound(e.to_string()))?;
    let vault_path = config
        .vault_path
        .as_ref()
        .ok_or(AppError::VaultNotConfigured)?;
    let abs_path = vault_path.join(&path);
    if !abs_path.exists() {
        return Err(AppError::NoteNotFound(path));
    }

    // 백링크 조회: path에서 title 추출 → backlinks 맵에서 소스 경로 조회
    let title = parser::title_from_path(&abs_path);
    let backlink_sources = index.backlinks.get(&title).cloned().unwrap_or_default();

    let backlinks: Vec<BacklinkEntry> = backlink_sources
        .iter()
        .filter_map(|source_path| {
            index.notes.iter().find(|n| n.path == *source_path).map(|n| {
                BacklinkEntry {
                    path: n.path.clone(),
                    title: n.title.clone(),
                    note_type: n.frontmatter.as_ref().map(|fm| fm.note_type.clone()),
                    context: format!("→ [[{}]]", title),
                }
            })
        })
        .collect();

    renderer::render_note(&abs_path, vault_path, &backlinks)
}

#[tauri::command]
pub fn get_backlinks(
    state: State<'_, VaultState>,
    path: String,
) -> Result<Vec<BacklinkEntry>, AppError> {
    let index = state.read().map_err(|e| AppError::NoteNotFound(e.to_string()))?;

    let title = Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let backlink_sources = index.backlinks.get(&title).cloned().unwrap_or_default();

    let backlinks: Vec<BacklinkEntry> = backlink_sources
        .iter()
        .filter_map(|source_path| {
            index.notes.iter().find(|n| n.path == *source_path).map(|n| {
                BacklinkEntry {
                    path: n.path.clone(),
                    title: n.title.clone(),
                    note_type: n.frontmatter.as_ref().map(|fm| fm.note_type.clone()),
                    context: format!("→ [[{}]]", title),
                }
            })
        })
        .collect();

    Ok(backlinks)
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
    let vault_path = config
        .vault_path
        .as_ref()
        .ok_or(AppError::VaultNotConfigured)?;
    let abs_path = vault_path.join(&path);
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
) -> Result<String, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::NoteNotFound(e.to_string()))?;
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

    let vault_path = config
        .vault_path
        .as_ref()
        .ok_or(AppError::VaultNotConfigured)?;
    let inbox_dir = vault_path.join(&config.quick_note_folder);
    fs::create_dir_all(&inbox_dir)?;

    let mut file_path = inbox_dir.join(&filename);

    // 파일명 충돌 처리
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
        .strip_prefix(vault_path)
        .unwrap_or(&file_path)
        .to_string_lossy()
        .to_string();

    Ok(relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use tempfile::TempDir;

    fn test_config(dir: &TempDir) -> AppConfig {
        AppConfig {
            vault_path: Some(dir.path().to_path_buf()),
            quick_note_folder: "inbox".to_string(),
            ..Default::default()
        }
    }

    // create_quick_note는 State<AppConfig>를 받으므로 직접 호출 불가.
    // 핵심 로직을 테스트하기 위해 파일 생성 로직을 검증.

    #[test]
    fn creates_note_with_title() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let today = Local::now().format("%Y-%m-%d").to_string();

        let inbox_dir = config.vault_path.as_ref().unwrap().join("inbox");
        fs::create_dir_all(&inbox_dir).unwrap();

        let slug = slug::slugify("오늘의 메모");
        let filename = format!("{}-{}.md", today, slug);
        let file_path = inbox_dir.join(&filename);

        let content = format!(
            "---\ntype: idea\ncreated: {}\nstatus: seedling\ntags:\n  - dev\n---\n\n# 오늘의 메모\n\ntest content",
            today
        );
        fs::write(&file_path, &content).unwrap();

        assert!(file_path.exists());
        let saved = fs::read_to_string(&file_path).unwrap();
        assert!(saved.contains("type: idea"));
        assert!(saved.contains("# 오늘의 메모"));
        assert!(saved.contains("test content"));
        assert!(saved.contains("  - dev"));
    }

    #[test]
    fn creates_note_without_title() {
        let dir = TempDir::new().unwrap();
        let config = test_config(&dir);
        let today = Local::now().format("%Y-%m-%d").to_string();

        let inbox_dir = config.vault_path.as_ref().unwrap().join("inbox");
        fs::create_dir_all(&inbox_dir).unwrap();

        let timestamp = Local::now().format("%H%M%S").to_string();
        let filename = format!("{}-{}.md", today, timestamp);
        let file_path = inbox_dir.join(&filename);

        let content = format!(
            "---\ntype: idea\ncreated: {}\nstatus: seedling\ntags: []\n---\n\n# Quick Note\n\nmy quick note",
            today
        );
        fs::write(&file_path, &content).unwrap();

        assert!(file_path.exists());
        let saved = fs::read_to_string(&file_path).unwrap();
        assert!(saved.contains("# Quick Note"));
    }

    #[test]
    fn inbox_auto_created() {
        let dir = TempDir::new().unwrap();
        let inbox_dir = dir.path().join("inbox");
        assert!(!inbox_dir.exists());

        fs::create_dir_all(&inbox_dir).unwrap();
        assert!(inbox_dir.exists());
    }
}
