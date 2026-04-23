//! 멀티 볼트 관리 커맨드 + 순수 헬퍼.
//!
//! - normalize_vault_path / derive_vault_name / upsert_vault: 순수 함수
//! - list_vaults / switch_vault / remove_vault: IPC 커맨드

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, State};

use crate::config::{save_config, ConfigState, VaultEntry};
use crate::error::AppError;
use crate::notifications::NotificationsState;
use crate::watcher::WatcherState;

use super::onboarding::{activate_vault, VaultStatus};

/// 경로를 정규화한다:
/// - trailing slash 제거
/// - 상대경로는 현재 작업 디렉토리 기준 절대경로로 변환
pub fn normalize_vault_path(input: &Path) -> PathBuf {
    let s = input.to_string_lossy();
    let trimmed = s.trim_end_matches('/');
    let as_path = if trimmed.is_empty() {
        PathBuf::from("/")
    } else {
        PathBuf::from(trimmed)
    };

    if as_path.is_absolute() {
        as_path
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&as_path))
            .unwrap_or(as_path)
    }
}

/// 폴더명에서 표시용 이름을 추출한다. 추출 실패 시 "vault" fallback.
pub fn derive_vault_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| "vault".to_string())
}

/// `vaults`에 중복 없이 `entry`를 추가한다. 동일 경로가 이미 있으면 no-op.
pub fn upsert_vault(vaults: &mut Vec<VaultEntry>, entry: VaultEntry) {
    if vaults.iter().any(|v| v.path == entry.path) {
        return;
    }
    vaults.push(entry);
}

/// 현재 활성 볼트가 `vaults`에 없으면 자동 보정 후 복제 반환.
/// (구버전 config migration 처리)
pub fn enriched_vaults_list(
    vault_path: Option<&PathBuf>,
    vaults: &mut Vec<VaultEntry>,
) -> Vec<VaultEntry> {
    if let Some(path) = vault_path {
        let normalized = normalize_vault_path(path);
        let entry = VaultEntry {
            name: derive_vault_name(&normalized),
            path: normalized,
        };
        upsert_vault(vaults, entry);
    }
    vaults.clone()
}

#[tauri::command]
pub fn list_vaults(
    config_state: State<'_, ConfigState>,
    app_handle: AppHandle,
) -> Result<Vec<VaultEntry>, AppError> {
    let (result, snapshot) = {
        let mut config = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        let prev_len = config.vaults.len();
        let vault_path = config.vault_path.clone();
        let result = enriched_vaults_list(vault_path.as_ref(), &mut config.vaults);
        let changed = config.vaults.len() != prev_len;
        (result, if changed { Some(config.clone()) } else { None })
    };

    if let Some(snapshot) = snapshot {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        save_config(&snapshot, &app_data_dir)?;
    }

    Ok(result)
}

#[tauri::command]
pub fn switch_vault(
    config_state: State<'_, ConfigState>,
    watcher_state: State<'_, WatcherState>,
    notifications_state: State<'_, NotificationsState>,
    app_handle: AppHandle,
    path: String,
) -> Result<VaultStatus, AppError> {
    let normalized = normalize_vault_path(Path::new(&path));

    // 목록에 있는 경로만 전환 허용
    {
        let config = config_state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        if !config.vaults.iter().any(|v| v.path == normalized) {
            return Err(AppError::VaultNotFound(path));
        }
    }

    activate_vault(
        &config_state,
        &watcher_state,
        &notifications_state,
        &app_handle,
        normalized,
    )
}

#[tauri::command]
pub fn remove_vault(
    config_state: State<'_, ConfigState>,
    app_handle: AppHandle,
    path: String,
) -> Result<Vec<VaultEntry>, AppError> {
    let normalized = normalize_vault_path(Path::new(&path));

    let snapshot = {
        let mut config = config_state
            .write()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;

        // 활성 볼트 제거 금지
        if config.vault_path.as_ref() == Some(&normalized) {
            return Err(AppError::VaultNotFound(
                "활성 볼트는 제거할 수 없음 (먼저 다른 볼트로 전환)".into(),
            ));
        }

        config.vaults.retain(|v| v.path != normalized);
        config.clone()
    };

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    save_config(&snapshot, &app_data_dir)?;

    Ok(snapshot.vaults)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- normalize_vault_path ---

    // BC #1: trailing slash 제거
    #[test]
    fn normalize_strips_trailing_slash() {
        assert_eq!(
            normalize_vault_path(Path::new("/foo/bar/")),
            PathBuf::from("/foo/bar")
        );
    }

    // BC #2: 이미 정규형
    #[test]
    fn normalize_already_canonical() {
        assert_eq!(
            normalize_vault_path(Path::new("/foo/bar")),
            PathBuf::from("/foo/bar")
        );
    }

    // BC #3: 상대경로 → 절대경로
    #[test]
    fn normalize_relative_to_absolute() {
        let result = normalize_vault_path(Path::new("./vault"));
        assert!(result.is_absolute());
        assert!(result.ends_with("vault"));
    }

    // BC #4: 절대경로 보존
    #[test]
    fn normalize_absolute_preserved() {
        assert_eq!(
            normalize_vault_path(Path::new("/absolute/path")),
            PathBuf::from("/absolute/path")
        );
    }

    // --- derive_vault_name ---

    // BC #5
    #[test]
    fn derive_name_from_basename() {
        assert_eq!(derive_vault_name(Path::new("/foo/my-vault")), "my-vault");
    }

    // BC #6 (정규화 이후 호출 상정이지만 방어적으로)
    #[test]
    fn derive_name_handles_trailing_slash() {
        // 정규화를 거치지 않아도 Path는 trailing slash 무관
        assert_eq!(derive_vault_name(Path::new("/foo/my-vault/")), "my-vault");
    }

    // BC #7
    #[test]
    fn derive_name_root_fallback() {
        assert_eq!(derive_vault_name(Path::new("/")), "vault");
    }

    // --- upsert_vault ---

    fn entry(path: &str, name: &str) -> VaultEntry {
        VaultEntry {
            path: PathBuf::from(path),
            name: name.to_string(),
        }
    }

    // BC #8
    #[test]
    fn upsert_adds_to_empty() {
        let mut vaults = Vec::new();
        upsert_vault(&mut vaults, entry("/a", "a"));
        assert_eq!(vaults.len(), 1);
    }

    // BC #9: 동일 경로 중복 방지 (이름 달라도)
    #[test]
    fn upsert_skips_duplicate_path() {
        let mut vaults = vec![entry("/a", "original")];
        upsert_vault(&mut vaults, entry("/a", "different-name"));
        assert_eq!(vaults.len(), 1);
        assert_eq!(vaults[0].name, "original"); // 첫 등장 유지
    }

    // BC #10: 다른 경로 추가
    #[test]
    fn upsert_appends_distinct_path() {
        let mut vaults = vec![entry("/a", "a")];
        upsert_vault(&mut vaults, entry("/b", "b"));
        assert_eq!(vaults.len(), 2);
    }

    // --- enriched_vaults_list (migration 로직) ---

    // 구버전 config: vault_path가 있으나 vaults는 비어있음 → 활성 볼트가 목록에 자동 보정
    #[test]
    fn enriched_list_auto_inserts_active_vault() {
        let mut vaults = Vec::new();
        let active = PathBuf::from("/active/vault");
        let result = enriched_vaults_list(Some(&active), &mut vaults);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, active);
        assert_eq!(result[0].name, "vault"); // "/active/vault" → "vault" (basename)

        // 원본 Vec도 함께 갱신
        assert_eq!(vaults.len(), 1);
    }

    // vault_path가 None이면 변경 없음
    #[test]
    fn enriched_list_no_active_no_change() {
        let mut vaults = vec![entry("/x", "x")];
        let result = enriched_vaults_list(None, &mut vaults);
        assert_eq!(result.len(), 1);
        assert_eq!(vaults.len(), 1);
    }

    // 활성 볼트가 이미 목록에 있음 → 중복 없이 유지
    #[test]
    fn enriched_list_active_already_present() {
        let mut vaults = vec![entry("/a", "alpha")];
        let result = enriched_vaults_list(Some(&PathBuf::from("/a")), &mut vaults);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "alpha"); // 기존 이름 유지
    }
}
