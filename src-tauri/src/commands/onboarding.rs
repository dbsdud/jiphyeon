use std::fs;
use std::path::{Path, PathBuf};

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

/// 볼트에 생성할 11개 디렉토리
pub(crate) const VAULT_DIRECTORIES: &[&str] = &[
    "inbox",
    "dev",
    "decisions",
    "readings",
    "meetings",
    "ideas",
    "artifacts",
    "projects",
    "_moc",
    "_templates",
    "_maintenance",
];

/// (상대 경로, 내용) 튜플로 생성할 파일 목록
pub(crate) fn vault_files() -> Vec<(&'static str, &'static str)> {
    macro_rules! tpl {
        ($rel:expr) => {
            ($rel, include_str!(concat!("../../templates/vault/", $rel)))
        };
    }
    vec![
        tpl!(".gitignore"),
        tpl!(".gitattributes"),
        tpl!("CLAUDE.md"),
        // MOC
        tpl!("_moc/Home.md"),
        tpl!("_moc/Topics.md"),
        tpl!("_moc/Projects.md"),
        tpl!("_moc/Timeline.md"),
        // 노트 템플릿
        tpl!("_templates/tpl-artifact.md"),
        tpl!("_templates/tpl-clipping.md"),
        tpl!("_templates/tpl-decision.md"),
        tpl!("_templates/tpl-idea.md"),
        tpl!("_templates/tpl-meeting.md"),
        tpl!("_templates/tpl-project-moc.md"),
        tpl!("_templates/tpl-reading.md"),
        tpl!("_templates/tpl-til.md"),
        tpl!("_templates/tpl-topic-moc.md"),
        // Claude Code 설정
        tpl!(".claude/settings.json"),
        // Claude Code hooks (실행 권한 자동 부여)
        tpl!(".claude/hooks/_notify.sh"),
        tpl!(".claude/hooks/check-broken-links.sh"),
        tpl!(".claude/hooks/check-filename.sh"),
        tpl!(".claude/hooks/check-orphan-note.sh"),
        tpl!(".claude/hooks/check-tag-duplication.sh"),
        tpl!(".claude/hooks/inject-date-tokens.sh"),
        tpl!(".claude/hooks/pre-validate-note.sh"),
        tpl!(".claude/hooks/protect-system-files.sh"),
        tpl!(".claude/hooks/review-reminder.sh"),
        tpl!(".claude/hooks/session-activity-log.sh"),
        tpl!(".claude/hooks/validate-frontmatter.sh"),
        tpl!(".claude/hooks/vault-health-snapshot.sh"),
        // Claude Code skills
        tpl!(".claude/skills/vault-archive/SKILL.md"),
        tpl!(".claude/skills/vault-audit/SKILL.md"),
        tpl!(".claude/skills/vault-clip/SKILL.md"),
        tpl!(".claude/skills/vault-daily/SKILL.md"),
        tpl!(".claude/skills/vault-gap/SKILL.md"),
        tpl!(".claude/skills/vault-graph/SKILL.md"),
        tpl!(".claude/skills/vault-link/SKILL.md"),
        tpl!(".claude/skills/vault-mature/SKILL.md"),
        tpl!(".claude/skills/vault-moc-sync/SKILL.md"),
        tpl!(".claude/skills/vault-new/SKILL.md"),
        tpl!(".claude/skills/vault-review/SKILL.md"),
        tpl!(".claude/skills/vault-search/SKILL.md"),
        tpl!(".claude/skills/vault-synthesize/SKILL.md"),
        tpl!(".claude/skills/vault-tags/SKILL.md"),
    ]
}

/// hook 쉘 스크립트는 실행 권한이 필요하다 (Unix 한정, Windows에서는 no-op)
pub(crate) fn is_executable_script(rel_path: &str) -> bool {
    rel_path.starts_with(".claude/hooks/") && rel_path.ends_with(".sh")
}

/// 새 볼트 생성 대상으로 안전한지 확인한다.
/// - 경로가 없거나 비어있으면 OK
/// - 이미 파일/디렉토리가 있으면 `VaultDirectoryNotEmpty`
fn ensure_empty_or_absent(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(path)?;
    if entries.next().is_none() {
        Ok(())
    } else {
        Err(AppError::VaultDirectoryNotEmpty(
            path.to_string_lossy().to_string(),
        ))
    }
}

/// 볼트 디렉토리 구조를 생성한다.
/// 기존 파일은 덮어쓰지 않으며, 누락된 것만 채운다.
pub fn scaffold_vault(root: &Path) -> Result<(), AppError> {
    if !root.exists() {
        return Err(AppError::VaultNotFound(root.to_string_lossy().to_string()));
    }

    for dir in VAULT_DIRECTORIES {
        fs::create_dir_all(root.join(dir))?;
    }

    for (rel_path, content) in vault_files() {
        let full_path = root.join(rel_path);
        if full_path.exists() {
            continue;
        }
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, content)?;

        if is_executable_script(rel_path) {
            set_executable(&full_path)?;
        }
    }

    Ok(())
}

#[cfg(unix)]
pub(crate) fn set_executable(path: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
pub(crate) fn set_executable(_path: &Path) -> Result<(), AppError> {
    Ok(())
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
pub fn create_vault(
    config_state: State<'_, ConfigState>,
    vault_state: State<'_, VaultState>,
    search_state: State<'_, search::SearchState>,
    watcher_state: State<'_, WatcherState>,
    notifications_state: State<'_, NotificationsState>,
    app_handle: AppHandle,
    path: String,
) -> Result<VaultStatus, AppError> {
    let vault_path = PathBuf::from(&path);
    ensure_empty_or_absent(&vault_path)?;
    fs::create_dir_all(&vault_path)?;
    scaffold_vault(&vault_path)?;

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
    // 0. 경로 정규화
    let vault_path = normalize_vault_path(&vault_path);

    // 1. 설정 업데이트 (vault_path + vaults upsert) + 영속화
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

    // 2. 인덱스 재구축
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

    // 3. watcher 재시작 (기존 핸들을 drop하여 자동 중지, 새 것으로 교체)
    //    watcher 내부에서 notifications_state.reset을 수행하므로 새 볼트 기준으로 자동 동기화됨.
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
    use tempfile::TempDir;

    // BC #1: 빈 디렉토리 → 11개 디렉토리 생성
    #[test]
    fn scaffold_creates_all_directories() {
        let dir = TempDir::new().unwrap();
        scaffold_vault(dir.path()).unwrap();

        for d in VAULT_DIRECTORIES {
            assert!(
                dir.path().join(d).is_dir(),
                "expected directory {} to exist",
                d
            );
        }
    }

    // BC #2: .gitignore, .gitattributes, CLAUDE.md 생성
    #[test]
    fn scaffold_creates_root_files() {
        let dir = TempDir::new().unwrap();
        scaffold_vault(dir.path()).unwrap();

        assert!(dir.path().join(".gitignore").is_file());
        assert!(dir.path().join(".gitattributes").is_file());
        assert!(dir.path().join("CLAUDE.md").is_file());
    }

    // BC #3: _moc/ 에 4개 파일 생성
    #[test]
    fn scaffold_creates_moc_files() {
        let dir = TempDir::new().unwrap();
        scaffold_vault(dir.path()).unwrap();

        for name in ["Home.md", "Topics.md", "Projects.md", "Timeline.md"] {
            assert!(dir.path().join("_moc").join(name).is_file());
        }
    }

    // .claude/ 설정과 hooks/skills 번들 생성
    #[test]
    fn scaffold_creates_claude_settings_and_hooks() {
        let dir = TempDir::new().unwrap();
        scaffold_vault(dir.path()).unwrap();

        assert!(dir.path().join(".claude/settings.json").is_file());
        assert!(dir.path().join(".claude/hooks/check-broken-links.sh").is_file());
        assert!(dir.path().join(".claude/hooks/validate-frontmatter.sh").is_file());
    }

    #[test]
    fn scaffold_creates_all_skills() {
        let dir = TempDir::new().unwrap();
        scaffold_vault(dir.path()).unwrap();

        let skills = [
            "vault-archive", "vault-audit", "vault-clip", "vault-daily", "vault-gap",
            "vault-graph", "vault-link", "vault-mature", "vault-moc-sync", "vault-new",
            "vault-review", "vault-search", "vault-synthesize", "vault-tags",
        ];
        for s in skills {
            let path = dir.path().join(".claude/skills").join(s).join("SKILL.md");
            assert!(path.is_file(), "expected skill {} to exist", s);
        }
    }

    // Unix 한정: hook 스크립트는 실행 권한이 있어야 함
    #[cfg(unix)]
    #[test]
    fn scaffold_sets_hooks_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        scaffold_vault(dir.path()).unwrap();

        let hook = dir.path().join(".claude/hooks/validate-frontmatter.sh");
        let mode = fs::metadata(&hook).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "owner+group+other execute bits expected");
    }

    // BC #4: _templates/ 에 9개 파일 생성
    #[test]
    fn scaffold_creates_template_files() {
        let dir = TempDir::new().unwrap();
        scaffold_vault(dir.path()).unwrap();

        let templates = [
            "tpl-artifact.md",
            "tpl-clipping.md",
            "tpl-decision.md",
            "tpl-idea.md",
            "tpl-meeting.md",
            "tpl-project-moc.md",
            "tpl-reading.md",
            "tpl-til.md",
            "tpl-topic-moc.md",
        ];
        for name in templates {
            assert!(
                dir.path().join("_templates").join(name).is_file(),
                "expected _templates/{} to exist",
                name
            );
        }
    }

    // BC #5: 이미 파일이 있으면 기존 내용 보존, 없는 것만 생성
    #[test]
    fn scaffold_preserves_existing_files() {
        let dir = TempDir::new().unwrap();
        let claude_md = dir.path().join("CLAUDE.md");
        fs::write(&claude_md, "USER CONTENT").unwrap();

        scaffold_vault(dir.path()).unwrap();

        assert_eq!(fs::read_to_string(&claude_md).unwrap(), "USER CONTENT");
        // 다른 파일은 여전히 생성되어야 함
        assert!(dir.path().join("_moc/Home.md").is_file());
    }

    // BC #6: 존재하지 않는 경로 → VaultNotFound
    #[test]
    fn scaffold_errors_when_root_missing() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("does-not-exist");

        let err = scaffold_vault(&missing).unwrap_err();
        assert!(matches!(err, AppError::VaultNotFound(_)));
    }

    // BC #7: vault_path == None → connected: false
    #[test]
    fn status_from_none_is_disconnected() {
        let status = status_from_path(None);
        assert!(!status.connected);
        assert!(status.vault_path.is_none());
    }

    // BC #8: vault_path == Some → connected: true
    #[test]
    fn status_from_some_is_connected() {
        let path = PathBuf::from("/tmp/my-vault");
        let status = status_from_path(Some(&path));
        assert!(status.connected);
        assert_eq!(status.vault_path.as_deref(), Some("/tmp/my-vault"));
    }

    // 존재하지 않는 경로 → OK (부모만 있으면 create_dir_all이 처리)
    #[test]
    fn ensure_empty_or_absent_allows_missing_path() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("new-vault");
        assert!(ensure_empty_or_absent(&missing).is_ok());
    }

    // 존재하지만 비어있는 경로 → OK
    #[test]
    fn ensure_empty_or_absent_allows_empty_dir() {
        let dir = TempDir::new().unwrap();
        assert!(ensure_empty_or_absent(dir.path()).is_ok());
    }

    // 이미 파일이 들어있는 경로 → VaultDirectoryNotEmpty (사용자 실수 방지)
    #[test]
    fn ensure_empty_or_absent_rejects_non_empty_dir() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.md"), "user content").unwrap();

        let err = ensure_empty_or_absent(dir.path()).unwrap_err();
        assert!(matches!(err, AppError::VaultDirectoryNotEmpty(_)));
    }

    // 숨김 파일 하나만 있어도 거부 (.DS_Store 등도 실수 신호로 보수적 처리)
    #[test]
    fn ensure_empty_or_absent_rejects_even_hidden_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".DS_Store"), "").unwrap();

        assert!(matches!(
            ensure_empty_or_absent(dir.path()),
            Err(AppError::VaultDirectoryNotEmpty(_))
        ));
    }
}
