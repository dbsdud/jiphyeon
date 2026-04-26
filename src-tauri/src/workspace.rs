//! v2.5 Workspace Hub 헬퍼.
//!
//! 사용자 홈 아래 단일 디렉토리(`~/Jiphyeon/`)에 등록된 각 프로젝트를 symlink 로
//! 노출하여, 집현·터미널·Finder 가 모두 같은 hub 경로에서 작업할 수 있게 한다.
//!
//! Windows 는 `std::os::windows::fs::symlink_dir` 권한 이슈로 이 모듈은 unix 한정.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;

const MAX_NAME_SUFFIX: usize = 1000;

/// 기본 hub 경로: `$HOME/Jiphyeon`. HOME 미해석 시 cwd 기준 `./Jiphyeon` 폴백.
pub fn default_workspace_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join("Jiphyeon"))
        .unwrap_or_else(|| PathBuf::from("./Jiphyeon"))
}

/// hub 디렉토리 보장. 없으면 생성, 일반 파일이 점유하면 에러.
pub fn ensure_workspace_dir(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        if path.is_dir() {
            return Ok(());
        }
        return Err(AppError::InvalidPath(path.to_string_lossy().to_string()));
    }
    fs::create_dir_all(path)?;
    Ok(())
}

/// `<workspace>/<requested_name>` 에 `target` symlink 를 생성한다.
/// - 이미 동일 target 의 symlink 가 있으면 그 경로를 그대로 반환 (idempotent)
/// - 다른 점유물(다른 symlink, 일반 폴더/파일) 충돌 시 `<name>-2`, `<name>-3` ... 순으로 시도
/// - target 이 존재하지 않으면 `AppError::VaultNotFound`
pub fn create_project_symlink(
    workspace: &Path,
    requested_name: &str,
    target: &Path,
) -> Result<PathBuf, AppError> {
    if !target.exists() {
        return Err(AppError::VaultNotFound(target.to_string_lossy().to_string()));
    }

    let base = sanitize_name(requested_name);
    for n in 0..=MAX_NAME_SUFFIX {
        let candidate_name = if n == 0 {
            base.clone()
        } else {
            format!("{}-{}", base, n + 1)
        };
        let link = workspace.join(&candidate_name);

        match link.symlink_metadata() {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                create_symlink(target, &link)?;
                return Ok(link);
            }
            Err(e) => return Err(AppError::Io(e)),
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    if let Ok(existing_target) = fs::read_link(&link) {
                        if path_equal(&existing_target, target) {
                            return Ok(link);
                        }
                    }
                }
                // 점유 중 → suffix 시도
            }
        }
    }
    Err(AppError::InvalidPath(format!(
        "hub 내 사용 가능한 이름을 찾지 못함: {}",
        base
    )))
}

/// link 가 깨졌는지 검사. 일반 파일/디렉토리 또는 미존재면 true.
pub fn is_link_broken(link_path: &Path) -> bool {
    let Ok(meta) = link_path.symlink_metadata() else {
        return true;
    };
    if !meta.file_type().is_symlink() {
        return true;
    }
    // symlink 가 있어도 target 이 사라지면 exists() == false
    !link_path.exists()
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<(), AppError> {
    std::os::unix::fs::symlink(target, link).map_err(AppError::Io)
}

#[cfg(not(unix))]
fn create_symlink(_target: &Path, _link: &Path) -> Result<(), AppError> {
    Err(AppError::InvalidPath(
        "심볼릭 링크는 macOS/Linux 에서만 지원됩니다".to_string(),
    ))
}

/// 경로 비교 (canonicalize 안 함; symlink target 비교는 lexical).
fn path_equal(a: &Path, b: &Path) -> bool {
    a == b
}

/// 폴더명에 쓸 수 없는 문자(`/`, `\0`)를 `_` 로 치환. 빈 문자열은 "project" 폴백.
fn sanitize_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "project".to_string();
    }
    trimmed
        .chars()
        .map(|c| if c == '/' || c == '\0' { '_' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- default_workspace_path ---

    #[test]
    fn default_workspace_uses_home_jiphyeon() {
        let path = default_workspace_path();
        assert!(path.ends_with("Jiphyeon"));
    }

    // --- ensure_workspace_dir ---

    #[test]
    fn ensure_creates_missing_dir() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("hub");
        assert!(!target.exists());
        ensure_workspace_dir(&target).unwrap();
        assert!(target.is_dir());
    }

    #[test]
    fn ensure_noop_for_existing_dir() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("hub");
        fs::create_dir_all(&target).unwrap();
        ensure_workspace_dir(&target).unwrap(); // no panic
    }

    #[test]
    fn ensure_errors_when_path_is_file() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("hub");
        fs::write(&target, "not a dir").unwrap();
        let err = ensure_workspace_dir(&target);
        assert!(matches!(err, Err(AppError::InvalidPath(_))));
    }

    // --- create_project_symlink ---

    fn make_target(parent: &Path, name: &str) -> PathBuf {
        let p = parent.join(name);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn symlink_creates_link_in_workspace() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("hub");
        fs::create_dir_all(&workspace).unwrap();
        let target = make_target(dir.path(), "real");

        let link = create_project_symlink(&workspace, "real", &target).unwrap();
        assert_eq!(link, workspace.join("real"));
        assert!(link.exists());
        let resolved = fs::read_link(&link).unwrap();
        assert_eq!(resolved, target);
    }

    #[test]
    fn symlink_idempotent_when_same_target() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("hub");
        fs::create_dir_all(&workspace).unwrap();
        let target = make_target(dir.path(), "real");

        let first = create_project_symlink(&workspace, "real", &target).unwrap();
        let second = create_project_symlink(&workspace, "real", &target).unwrap();
        assert_eq!(first, second);
        assert_eq!(first, workspace.join("real"));
    }

    #[test]
    fn symlink_suffix_when_other_target_collides() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("hub");
        fs::create_dir_all(&workspace).unwrap();
        let target_a = make_target(dir.path(), "real-a");
        let target_b = make_target(dir.path(), "real-b");

        let link_a = create_project_symlink(&workspace, "shared", &target_a).unwrap();
        let link_b = create_project_symlink(&workspace, "shared", &target_b).unwrap();

        assert_eq!(link_a, workspace.join("shared"));
        assert_eq!(link_b, workspace.join("shared-2"));
    }

    #[test]
    fn symlink_suffix_when_plain_dir_occupies_name() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("hub");
        fs::create_dir_all(&workspace).unwrap();
        // 일반 폴더로 이름 점유 (사용자가 직접 만든 무관 폴더)
        fs::create_dir_all(workspace.join("foo")).unwrap();
        let target = make_target(dir.path(), "real");

        let link = create_project_symlink(&workspace, "foo", &target).unwrap();
        assert_eq!(link, workspace.join("foo-2"));
    }

    #[test]
    fn symlink_errors_when_target_missing() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("hub");
        fs::create_dir_all(&workspace).unwrap();
        let err = create_project_symlink(&workspace, "ghost", &dir.path().join("nope"));
        assert!(matches!(err, Err(AppError::VaultNotFound(_))));
    }

    #[test]
    fn symlink_sanitizes_slash_in_name() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("hub");
        fs::create_dir_all(&workspace).unwrap();
        let target = make_target(dir.path(), "real");

        let link = create_project_symlink(&workspace, "weird/name", &target).unwrap();
        assert_eq!(link, workspace.join("weird_name"));
    }

    // --- is_link_broken ---

    #[test]
    fn broken_returns_true_for_missing_path() {
        let dir = TempDir::new().unwrap();
        assert!(is_link_broken(&dir.path().join("nope")));
    }

    #[test]
    fn broken_returns_true_for_plain_dir() {
        let dir = TempDir::new().unwrap();
        let plain = dir.path().join("plain");
        fs::create_dir_all(&plain).unwrap();
        assert!(is_link_broken(&plain));
    }

    #[test]
    fn broken_returns_false_for_valid_symlink() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("hub");
        fs::create_dir_all(&workspace).unwrap();
        let target = make_target(dir.path(), "real");
        let link = create_project_symlink(&workspace, "real", &target).unwrap();
        assert!(!is_link_broken(&link));
    }

    #[test]
    fn broken_returns_true_when_target_removed() {
        let dir = TempDir::new().unwrap();
        let workspace = dir.path().join("hub");
        fs::create_dir_all(&workspace).unwrap();
        let target = make_target(dir.path(), "real");
        let link = create_project_symlink(&workspace, "real", &target).unwrap();
        fs::remove_dir(&target).unwrap();
        assert!(is_link_broken(&link));
    }
}
