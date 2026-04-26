//! Project 레지스트리 IPC 커맨드.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, ConfigState};
use crate::error::AppError;
use crate::notifications::NotificationsState;
use crate::project::{
    derive_project_id, derive_project_name, new_project_entry, normalize_root,
    read_last_graphify_at, ProjectEntry,
};
use crate::watcher::{self, WatcherState};

/// 폴더 등록 전 사전 점검 결과.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectInspection {
    pub root_path: String,
    pub root_exists: bool,
    pub docs_exists: bool,
    pub docs_is_dir: bool,
    pub graphify_out_exists: bool,
    pub already_registered: bool,
    pub suggested_name: String,
}

#[tauri::command]
pub fn inspect_project_root(
    config_state: State<'_, ConfigState>,
    root_path: String,
) -> Result<ProjectInspection, AppError> {
    let raw = PathBuf::from(&root_path);
    let normalized = normalize_root(&raw);
    let already_registered = {
        let config = config_state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        let id = derive_project_id(&normalized);
        config.projects.iter().any(|p| p.id == id)
    };
    Ok(inspect_path(&normalized, already_registered))
}

fn inspect_path(normalized: &Path, already_registered: bool) -> ProjectInspection {
    let root_exists = normalized.exists();
    let docs = normalized.join("docs");
    let docs_meta = fs::metadata(&docs).ok();
    let docs_exists = docs_meta.is_some();
    let docs_is_dir = docs_meta.map(|m| m.is_dir()).unwrap_or(false);
    let graphify_out_exists = normalized.join("graphify-out").is_dir();

    ProjectInspection {
        root_path: normalized.to_string_lossy().to_string(),
        root_exists,
        docs_exists,
        docs_is_dir,
        graphify_out_exists,
        already_registered,
        suggested_name: derive_project_name(normalized),
    }
}

#[tauri::command]
pub fn list_projects(
    config_state: State<'_, ConfigState>,
) -> Result<Vec<ProjectEntry>, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(config
        .projects
        .iter()
        .cloned()
        .map(refresh_last_graphify)
        .collect())
}

#[tauri::command]
pub fn get_active_project(
    config_state: State<'_, ConfigState>,
) -> Result<Option<ProjectEntry>, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(config.active_project().cloned().map(refresh_last_graphify))
}

#[tauri::command]
pub fn register_project(
    config_state: State<'_, ConfigState>,
    watcher_state: State<'_, WatcherState>,
    notifications_state: State<'_, NotificationsState>,
    app_handle: AppHandle,
    root_path: String,
    name: Option<String>,
    create_docs: bool,
) -> Result<ProjectEntry, AppError> {
    let raw = PathBuf::from(&root_path);
    if !raw.exists() {
        return Err(AppError::VaultNotFound(root_path));
    }
    let normalized = normalize_root(&raw);

    // docs/ 처리
    let docs_dir = normalized.join("docs");
    if !docs_dir.exists() {
        if create_docs {
            fs::create_dir_all(&docs_dir)?;
        } else {
            return Err(AppError::NoteNotFound(format!(
                "docs/ 가 존재하지 않음: {}",
                docs_dir.display()
            )));
        }
    } else if !docs_dir.is_dir() {
        return Err(AppError::InvalidPath(docs_dir.to_string_lossy().to_string()));
    }

    let new_id = derive_project_id(&normalized);

    let entry: ProjectEntry = {
        let mut config = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;

        if let Some(existing) = config.projects.iter().find(|p| p.id == new_id).cloned() {
            // 동일 root 재등록 → 활성만 전환
            config.active_project_id = Some(existing.id.clone());
            existing
        } else {
            let entry = new_project_entry(normalized.clone(), name);
            config.projects.push(entry.clone());
            config.active_project_id = Some(entry.id.clone());
            entry
        }
    };

    persist_and_restart_watcher(
        &config_state,
        &watcher_state,
        &notifications_state,
        &app_handle,
    )?;

    Ok(refresh_last_graphify(entry))
}

#[tauri::command]
pub fn switch_project(
    config_state: State<'_, ConfigState>,
    watcher_state: State<'_, WatcherState>,
    notifications_state: State<'_, NotificationsState>,
    app_handle: AppHandle,
    id: String,
) -> Result<ProjectEntry, AppError> {
    let entry = {
        let mut config = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        let Some(found) = config.projects.iter().find(|p| p.id == id).cloned() else {
            return Err(AppError::VaultNotFound(id));
        };
        config.active_project_id = Some(found.id.clone());
        found
    };

    persist_and_restart_watcher(
        &config_state,
        &watcher_state,
        &notifications_state,
        &app_handle,
    )?;

    Ok(refresh_last_graphify(entry))
}

#[tauri::command]
pub fn remove_project(
    config_state: State<'_, ConfigState>,
    app_handle: AppHandle,
    id: String,
) -> Result<Vec<ProjectEntry>, AppError> {
    let snapshot = {
        let mut config = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        if config.active_project_id.as_deref() == Some(id.as_str()) {
            return Err(AppError::VaultNotFound(
                "활성 프로젝트는 제거할 수 없음 (먼저 다른 프로젝트로 전환)".into(),
            ));
        }
        config.projects.retain(|p| p.id != id);
        config.clone()
    };

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    save_config(&snapshot, &app_data_dir)?;

    Ok(snapshot
        .projects
        .into_iter()
        .map(refresh_last_graphify)
        .collect())
}

/// 호출 시점의 graph.json mtime 으로 last_graphify_at 재계산.
fn refresh_last_graphify(mut entry: ProjectEntry) -> ProjectEntry {
    entry.last_graphify_at = read_last_graphify_at(&entry.graphify_out_path);
    entry
}

/// config 저장 + 활성 프로젝트 root 기준으로 watcher 재시작.
/// 활성 프로젝트가 없으면 watcher 미기동.
fn persist_and_restart_watcher(
    config_state: &ConfigState,
    watcher_state: &WatcherState,
    notifications_state: &NotificationsState,
    app_handle: &AppHandle,
) -> Result<(), AppError> {
    let config_snapshot = {
        let guard = config_state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        guard.clone()
    };
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    save_config(&config_snapshot, &app_data_dir)?;

    let watch_root = config_snapshot.active_project().map(|p| p.root_path.clone());

    {
        let mut guard = watcher_state
            .lock()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        *guard = None; // 기존 watcher drop
    }

    if let Some(root) = watch_root {
        let new_watcher = watcher::start_watching(
            app_handle.clone(),
            &root,
            &config_snapshot.exclude_dirs,
            config_snapshot.watch_debounce_ms,
            (*notifications_state).clone(),
        )?;
        let mut guard = watcher_state
            .lock()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        *guard = Some(new_watcher);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use tempfile::TempDir;

    // --- inspect_path ---

    #[test]
    fn inspect_missing_root() {
        let dir = TempDir::new().unwrap();
        let nonexistent = dir.path().join("ghost");
        let r = inspect_path(&nonexistent, false);
        assert!(!r.root_exists);
        assert!(!r.docs_exists);
        assert!(!r.graphify_out_exists);
        assert!(!r.already_registered);
        assert_eq!(r.suggested_name, "ghost");
    }

    #[test]
    fn inspect_root_without_docs_or_graphify() {
        let dir = TempDir::new().unwrap();
        let r = inspect_path(dir.path(), false);
        assert!(r.root_exists);
        assert!(!r.docs_exists);
        assert!(!r.docs_is_dir);
        assert!(!r.graphify_out_exists);
    }

    #[test]
    fn inspect_root_with_docs_as_file() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("docs"), "not a dir").unwrap();
        let r = inspect_path(dir.path(), false);
        assert!(r.root_exists);
        assert!(r.docs_exists);
        assert!(!r.docs_is_dir);
    }

    #[test]
    fn inspect_root_with_docs_and_graphify() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::create_dir_all(dir.path().join("graphify-out")).unwrap();
        let r = inspect_path(dir.path(), false);
        assert!(r.root_exists);
        assert!(r.docs_exists);
        assert!(r.docs_is_dir);
        assert!(r.graphify_out_exists);
    }

    #[test]
    fn inspect_already_registered_flag_pass_through() {
        let dir = TempDir::new().unwrap();
        let r = inspect_path(dir.path(), true);
        assert!(r.already_registered);
    }

    #[test]
    fn inspect_suggested_name_uses_basename() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("my-proj");
        fs::create_dir_all(&sub).unwrap();
        let r = inspect_path(&sub, false);
        assert_eq!(r.suggested_name, "my-proj");
    }

    #[test]
    fn refresh_sets_last_graphify_when_present() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("graphify-out")).unwrap();
        fs::write(dir.path().join("graphify-out/graph.json"), "{}").unwrap();

        let entry = new_project_entry(dir.path().to_path_buf(), None);
        let refreshed = refresh_last_graphify(entry);
        assert!(refreshed.last_graphify_at.is_some());
    }

    #[test]
    fn refresh_keeps_none_when_absent() {
        let dir = TempDir::new().unwrap();
        let entry = new_project_entry(dir.path().to_path_buf(), None);
        let refreshed = refresh_last_graphify(entry);
        assert!(refreshed.last_graphify_at.is_none());
    }

    #[test]
    fn active_project_resolution_via_config() {
        let dir = TempDir::new().unwrap();
        let entry = new_project_entry(dir.path().to_path_buf(), None);
        let id = entry.id.clone();
        let config = AppConfig {
            projects: vec![entry.clone()],
            active_project_id: Some(id.clone()),
            ..Default::default()
        };
        assert_eq!(config.active_project().map(|p| p.id.clone()), Some(id));
    }
}
