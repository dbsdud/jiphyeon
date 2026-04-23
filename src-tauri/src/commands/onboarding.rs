use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, ConfigState, VaultEntry};
use crate::error::AppError;
use crate::notifications::NotificationsState;
use crate::vault::{indexer, search};
use crate::watcher::{self, WatcherState};

use super::vault::VaultState;
use super::vaults::{derive_vault_name, normalize_vault_path, upsert_vault};

/// 볼트 연결 상태
#[derive(Debug, Serialize)]
pub struct VaultStatus {
    pub connected: bool,
    pub vault_path: Option<String>,
}

#[tauri::command]
pub fn get_vault_status(config_state: State<'_, ConfigState>) -> Result<VaultStatus, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(status_from_path(config.vault_path.as_ref()))
}

fn status_from_path(path: Option<&PathBuf>) -> VaultStatus {
    match path {
        Some(p) => VaultStatus {
            connected: true,
            vault_path: Some(p.to_string_lossy().to_string()),
        },
        None => VaultStatus {
            connected: false,
            vault_path: None,
        },
    }
}

#[tauri::command]
pub fn connect_vault(
    config_state: State<'_, ConfigState>,
    vault_state: State<'_, VaultState>,
    search_state: State<'_, search::SearchState>,
    watcher_state: State<'_, WatcherState>,
    notifications_state: State<'_, NotificationsState>,
    app_handle: AppHandle,
    path: String,
) -> Result<VaultStatus, AppError> {
    let vault_path = PathBuf::from(&path);
    if !vault_path.exists() {
        return Err(AppError::VaultNotFound(path));
    }

    activate_vault(
        &config_state,
        &vault_state,
        &search_state,
        &watcher_state,
        &notifications_state,
        &app_handle,
        vault_path,
    )
}

/// 설정 저장 + 인덱스 재구축 + 검색 인덱스 재구축 + watcher 재시작 공통 처리.
/// switch_vault에서도 재사용되므로 pub(crate).
pub(crate) fn activate_vault(
    config_state: &ConfigState,
    vault_state: &VaultState,
    search_state: &search::SearchState,
    watcher_state: &WatcherState,
    notifications_state: &NotificationsState,
    app_handle: &AppHandle,
    vault_path: PathBuf,
) -> Result<VaultStatus, AppError> {
    let vault_path = normalize_vault_path(&vault_path);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let config_snapshot = {
        let mut config = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        config.vault_path = Some(vault_path.clone());
        upsert_vault(
            &mut config.vaults,
            VaultEntry {
                name: derive_vault_name(&vault_path),
                path: vault_path.clone(),
            },
        );
        config.clone()
    };
    save_config(&config_snapshot, &app_data_dir)?;

    let new_index = indexer::scan_vault(&vault_path, &config_snapshot.exclude_dirs)?;
    let new_search = search::build_search_index(&new_index.notes)
        .map_err(|e| AppError::Search(e.to_string()))?;

    {
        let mut vs = vault_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        *vs = new_index;
    }
    {
        let mut ss = search_state
            .write()
            .map_err(|e| AppError::Search(e.to_string()))?;
        *ss = new_search;
    }

    let new_watcher = watcher::start_watching(
        app_handle.clone(),
        &config_snapshot,
        (*notifications_state).clone(),
    )?;
    {
        let mut guard = watcher_state
            .lock()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        *guard = Some(new_watcher);
    }

    Ok(VaultStatus {
        connected: true,
        vault_path: Some(vault_path.to_string_lossy().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_from_none_is_disconnected() {
        let status = status_from_path(None);
        assert!(!status.connected);
        assert!(status.vault_path.is_none());
    }

    #[test]
    fn status_from_some_is_connected() {
        let path = PathBuf::from("/tmp/my-vault");
        let status = status_from_path(Some(&path));
        assert!(status.connected);
        assert_eq!(status.vault_path.as_deref(), Some("/tmp/my-vault"));
    }
}
