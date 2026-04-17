use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, AppConfig, AppConfigPatch, ConfigState};
use crate::editor::{detect_editors as detect_editors_impl, DetectedEditor};
use crate::error::AppError;

/// 현재 설정 스냅샷 (ConfigState 읽기 clone).
#[tauri::command]
pub fn get_config(config_state: State<'_, ConfigState>) -> Result<AppConfig, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(config.clone())
}

/// Patch 머지 → save_config 성공 시에만 메모리 반영 (원자성).
#[tauri::command]
pub fn update_config(
    config_state: State<'_, ConfigState>,
    app_handle: AppHandle,
    patch: AppConfigPatch,
) -> Result<AppConfig, AppError> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));

    // 1) 현재 설정 스냅샷
    let current = {
        let guard = config_state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        guard.clone()
    };

    // 2) 머지
    let merged = current.merged_with(patch);

    // 3) 저장 먼저 — 실패하면 메모리 미반영 (원자성)
    save_config(&merged, &app_data_dir)?;

    // 4) 성공 시 메모리 반영
    {
        let mut guard = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        *guard = merged.clone();
    }

    Ok(merged)
}

/// 시스템에 설치된 에디터 감지 (플랫폼별 베스트 에포트).
#[tauri::command]
pub fn detect_editors() -> Vec<DetectedEditor> {
    detect_editors_impl()
}
