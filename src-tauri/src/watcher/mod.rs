use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use tauri::{AppHandle, Emitter};

use crate::config::AppConfig;
use crate::error::AppError;
use crate::models::{ChangeKind, VaultChangeEvent};
use crate::notifications::{collect_new_events, NotificationsState};

/// 런타임에 교체 가능한 watcher 핸들 슬롯.
/// 볼트 전환 시 이전 watcher를 drop(= 자동 중지)하고 새 watcher로 교체.
pub type WatcherState = Arc<Mutex<Option<VaultWatcher>>>;

const WATCH_EXTENSIONS: &[&str] = &["md", "html"];
/// 알림 파일 화이트리스트 — 확장자/exclude_dirs 필터를 우회해 항상 감시한다.
const NOTIFICATIONS_RELATIVE: &str = ".claude/state/notifications.jsonl";

/// 경로가 감시 대상인지 판별.
/// - notifications.jsonl은 화이트리스트로 우선 허용
/// - 그 외에는 확장자(*.md, *.html) + exclude_dirs 필터
fn should_watch(path: &Path, vault_root: &Path, exclude_dirs: &[String]) -> bool {
    let relative = match path.strip_prefix(vault_root) {
        Ok(r) => r,
        Err(_) => return false,
    };

    // 알림 파일은 필터 우회
    if relative == Path::new(NOTIFICATIONS_RELATIVE) {
        return true;
    }

    let ext_match = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| WATCH_EXTENSIONS.contains(&e))
        .unwrap_or(false);

    if !ext_match {
        return false;
    }

    for component in relative.components() {
        if let std::path::Component::Normal(name) = component {
            if let Some(name_str) = name.to_str() {
                if exclude_dirs.iter().any(|d| d == name_str) {
                    return false;
                }
            }
        }
    }

    true
}

/// 파일 존재 여부 + 인덱스 존재 여부로 ChangeKind 판별
#[allow(dead_code)]
fn classify_change(exists_on_disk: bool, exists_in_index: bool) -> Option<ChangeKind> {
    match (exists_on_disk, exists_in_index) {
        (true, false) => Some(ChangeKind::Created),
        (true, true) => Some(ChangeKind::Modified),
        (false, true) => Some(ChangeKind::Deleted),
        (false, false) => None,
    }
}

/// 감시 핸들 (drop 시 자동 중지)
pub struct VaultWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
}

/// 볼트 감시 시작. `notifications_state`는 시작 시점에 현재 라인 수로 `reset`되어
/// 기존 notifications.jsonl 백로그가 발화되지 않도록 한다.
pub fn start_watching(
    app_handle: AppHandle,
    config: &AppConfig,
    notifications_state: NotificationsState,
) -> Result<VaultWatcher, AppError> {
    let vault_root = config
        .vault_path
        .clone()
        .ok_or(AppError::VaultNotConfigured)?;
    let exclude_dirs = config.exclude_dirs.clone();
    let debounce_ms = config.watch_debounce_ms;

    // 시작 시점 오프셋을 현재 라인 수로 동기화 (백로그 플러시 방지).
    {
        let mut guard = notifications_state
            .lock()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        guard.reset(&vault_root);
    }

    let closure_vault_root = vault_root.clone();
    let closure_state = notifications_state.clone();

    let mut debouncer = new_debouncer(
        Duration::from_millis(debounce_ms),
        move |results: notify_debouncer_mini::DebounceEventResult| {
            let events = match results {
                Ok(events) => events,
                Err(err) => {
                    eprintln!("watcher error: {err:?}");
                    return;
                }
            };

            let mut notifications_touched = false;
            for event in events {
                let path = &event.path;
                if !should_watch(path, &closure_vault_root, &exclude_dirs) {
                    continue;
                }

                let relative_path = path.strip_prefix(&closure_vault_root).unwrap_or(path);
                if relative_path == Path::new(NOTIFICATIONS_RELATIVE) {
                    notifications_touched = true;
                    continue;
                }

                let exists_on_disk = path.exists();
                // debouncer-mini는 이벤트 종류를 구분하지 않으므로
                // 디스크 존재 여부만으로 Created/Modified/Deleted 판별.
                let kind = if exists_on_disk {
                    ChangeKind::Modified
                } else {
                    ChangeKind::Deleted
                };

                let change = VaultChangeEvent {
                    kind,
                    path: relative_path.to_string_lossy().to_string(),
                };

                if let Err(e) = app_handle.emit("vault-changed", &change) {
                    eprintln!("emit error: {e:?}");
                }
            }

            if notifications_touched {
                for ev in collect_new_events(&closure_vault_root, &closure_state) {
                    if let Err(e) = app_handle.emit("notification", &ev) {
                        eprintln!("notification emit error: {e:?}");
                    }
                }
            }
        },
    )
    .map_err(|e| AppError::Io(std::io::Error::other(e)))?;

    debouncer
        .watcher()
        .watch(&vault_root, RecursiveMode::Recursive)
        .map_err(|e| AppError::Io(std::io::Error::other(e)))?;

    Ok(VaultWatcher {
        _debouncer: debouncer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn exclude_dirs() -> Vec<String> {
        vec![
            ".git".to_string(),
            "dashboard".to_string(),
            "_templates".to_string(),
        ]
    }

    // --- BC #1: .md 파일 → true ---
    #[test]
    fn test_should_watch_md_file() {
        let vault = PathBuf::from("/vault");
        let path = PathBuf::from("/vault/notes/hello.md");
        assert!(should_watch(&path, &vault, &exclude_dirs()));
    }

    // --- BC #2: .html 파일 → true ---
    #[test]
    fn test_should_watch_html_file() {
        let vault = PathBuf::from("/vault");
        let path = PathBuf::from("/vault/clips/page.html");
        assert!(should_watch(&path, &vault, &exclude_dirs()));
    }

    // --- BC #3: .txt 파일 → false ---
    #[test]
    fn test_should_watch_rejects_txt() {
        let vault = PathBuf::from("/vault");
        let path = PathBuf::from("/vault/notes/hello.txt");
        assert!(!should_watch(&path, &vault, &exclude_dirs()));
    }

    // --- BC #4: exclude_dirs 내 파일 → false ---
    #[test]
    fn test_should_watch_rejects_excluded_dir() {
        let vault = PathBuf::from("/vault");
        let path = PathBuf::from("/vault/.git/hooks/pre-commit.md");
        assert!(!should_watch(&path, &vault, &exclude_dirs()));
    }

    // --- Edge: 중첩 exclude_dir ---
    #[test]
    fn test_should_watch_rejects_nested_excluded_dir() {
        let vault = PathBuf::from("/vault");
        let path = PathBuf::from("/vault/sub/.git/note.md");
        assert!(!should_watch(&path, &vault, &exclude_dirs()));
    }

    // --- Edge: vault_root 바깥 경로 → false ---
    #[test]
    fn test_should_watch_rejects_outside_vault() {
        let vault = PathBuf::from("/vault");
        let path = PathBuf::from("/other/place/note.md");
        assert!(!should_watch(&path, &vault, &exclude_dirs()));
    }

    // --- v0.6: notifications.jsonl 화이트리스트 (.claude 제외 필터 우회) ---
    #[test]
    fn test_should_watch_allows_notifications_jsonl() {
        let vault = PathBuf::from("/vault");
        let path = PathBuf::from("/vault/.claude/state/notifications.jsonl");
        // exclude_dirs에 ".claude"가 있지만 화이트리스트 경로라 허용
        let excludes = vec![".claude".to_string(), ".git".to_string()];
        assert!(should_watch(&path, &vault, &excludes));
    }

    // 다른 jsonl 파일은 확장자 필터에 걸려 제외
    #[test]
    fn test_should_watch_rejects_other_jsonl() {
        let vault = PathBuf::from("/vault");
        let path = PathBuf::from("/vault/logs/other.jsonl");
        assert!(!should_watch(&path, &vault, &exclude_dirs()));
    }

    // --- BC #5: 디스크 존재 + 인덱스 미존재 → Created ---
    #[test]
    fn test_classify_change_created() {
        assert_eq!(
            classify_change(true, false),
            Some(ChangeKind::Created)
        );
    }

    // --- BC #6: 디스크 존재 + 인덱스 존재 → Modified ---
    #[test]
    fn test_classify_change_modified() {
        assert_eq!(
            classify_change(true, true),
            Some(ChangeKind::Modified)
        );
    }

    // --- BC #7: 디스크 미존재 + 인덱스 존재 → Deleted ---
    #[test]
    fn test_classify_change_deleted() {
        assert_eq!(
            classify_change(false, true),
            Some(ChangeKind::Deleted)
        );
    }

    // --- BC #8: 디스크 미존재 + 인덱스 미존재 → None ---
    #[test]
    fn test_classify_change_none() {
        assert!(classify_change(false, false).is_none());
    }
}
