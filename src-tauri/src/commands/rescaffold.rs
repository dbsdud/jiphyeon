use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::State;

use super::onboarding::{is_executable_script, set_executable, vault_files, VAULT_DIRECTORIES};
use crate::config::{AppConfig, ConfigState};
use crate::error::AppError;

/// 재스캐폴드 모드. IPC 경계에서 kebab-case JSON으로 전달된다.
/// - `AddMissing` (add-missing): 누락된 파일만 생성. 기존 파일은 절대 건드리지 않는다.
/// - `ForceClaude` (force-claude): 누락은 생성 + `.claude/` 하위 템플릿과 다른 파일은 덮어쓴다.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RescaffoldMode {
    AddMissing,
    ForceClaude,
}

/// 재스캐폴드 실행 결과 리포트. dry-run에서도 동일 구조로 반환된다.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RescaffoldReport {
    /// 새로 생성된(또는 생성 예정인) 파일 (볼트 기준 상대경로)
    pub created: Vec<String>,
    /// 덮어쓴(또는 덮어쓸 예정인) `.claude/` 하위 파일
    pub overwritten: Vec<String>,
    /// 덮어쓰기 대상 중 사용자가 수정한 것으로 감지된 파일 (ForceClaude에서만 채워짐)
    pub modified_by_user: Vec<String>,
    /// 건드리지 않은 템플릿 파일 수 (= Skip)
    pub unchanged: usize,
    /// dry_run 여부 (UI에서 "미리보기" 결과와 "적용" 결과 구분용)
    pub dry_run: bool,
}

/// 특정 템플릿 파일에 대해 수행할 액션 (순수 계산 결과).
#[derive(Debug, PartialEq, Eq)]
pub enum FileAction {
    /// 디스크에 없음 → 새로 생성
    Create,
    /// 디스크에 있고 템플릿과 다름 + ForceClaude + `.claude/` 하위 → 덮어씀.
    /// `user_modified`는 사용자가 수정한 파일을 덮어쓰는 경우 true.
    Overwrite { user_modified: bool },
    /// 건드리지 않음
    Skip,
}

/// 디스크 파일의 바이트와 템플릿 바이트가 동일한지 비교.
/// 파일이 없거나 읽기 실패 시 false (= 보수적으로 "다르다"로 판정해 ForceClaude에서 덮어쓰기 후보가 됨).
pub fn matches_template(disk_path: &Path, template_content: &[u8]) -> bool {
    match fs::read(disk_path) {
        Ok(content) => content == template_content,
        Err(_) => false,
    }
}

/// 템플릿 파일 1건에 대해 모드/상태를 보고 액션을 결정.
///
/// 순수 함수 — I/O 없음. rel_path는 볼트 루트 기준 상대경로(e.g. ".claude/settings.json").
pub fn plan_action(
    rel_path: &str,
    mode: RescaffoldMode,
    disk_exists: bool,
    disk_matches_template: bool,
) -> FileAction {
    if !disk_exists {
        return FileAction::Create;
    }

    let is_system_asset = rel_path.starts_with(".claude/");
    match (mode, is_system_asset, disk_matches_template) {
        (RescaffoldMode::ForceClaude, true, false) => FileAction::Overwrite { user_modified: true },
        _ => FileAction::Skip,
    }
}

/// 볼트에 대해 재스캐폴드를 수행한다 (dry_run=true면 계산만, 파일 변경 없음).
///
/// - 루트가 없으면 `AppError::VaultNotFound`
/// - 디렉토리 11개는 dry_run이 아닐 때 idempotent하게 create_dir_all
/// - 템플릿 파일마다 `plan_action`으로 분기:
///   - Create → 생성 + hook이면 chmod +x
///   - Overwrite → 덮어쓰기 + hook이면 chmod +x + modified_by_user 기록
///   - Skip → unchanged 카운트 증가
pub fn rescaffold_vault(
    root: &Path,
    mode: RescaffoldMode,
    dry_run: bool,
) -> Result<RescaffoldReport, AppError> {
    if !root.exists() {
        return Err(AppError::VaultNotFound(root.to_string_lossy().to_string()));
    }

    if !dry_run {
        for dir in VAULT_DIRECTORIES {
            fs::create_dir_all(root.join(dir))?;
        }
    }

    let mut report = RescaffoldReport {
        dry_run,
        ..Default::default()
    };

    for (rel_path, content) in vault_files() {
        let full_path = root.join(rel_path);
        let disk_exists = full_path.exists();
        let matches = disk_exists && matches_template(&full_path, content.as_bytes());
        let action = plan_action(rel_path, mode, disk_exists, matches);

        match action {
            FileAction::Create => {
                if !dry_run {
                    if let Some(parent) = full_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&full_path, content)?;
                    if is_executable_script(rel_path) {
                        set_executable(&full_path)?;
                    }
                }
                report.created.push(rel_path.to_string());
            }
            FileAction::Overwrite { user_modified } => {
                if !dry_run {
                    fs::write(&full_path, content)?;
                    if is_executable_script(rel_path) {
                        set_executable(&full_path)?;
                    }
                }
                report.overwritten.push(rel_path.to_string());
                if user_modified {
                    report.modified_by_user.push(rel_path.to_string());
                }
            }
            FileAction::Skip => {
                report.unchanged += 1;
            }
        }
    }

    Ok(report)
}

/// IPC 본체 로직 (State에서 AppConfig를 꺼낸 뒤 위임).
/// 테스트에서 State를 만들기 어려워 분리한 얇은 헬퍼.
fn rescaffold_active_vault_impl(
    config: &AppConfig,
    mode: RescaffoldMode,
    dry_run: bool,
) -> Result<RescaffoldReport, AppError> {
    let vault_path = config
        .vault_path
        .as_ref()
        .ok_or(AppError::VaultNotConfigured)?;
    rescaffold_vault(vault_path, mode, dry_run)
}

/// 현재 활성 볼트에 대해 재스캐폴드 실행.
/// `vault_path`가 설정되어 있지 않으면 `VaultNotConfigured`.
#[tauri::command]
pub fn rescaffold_active_vault(
    config_state: State<'_, ConfigState>,
    mode: RescaffoldMode,
    dry_run: bool,
) -> Result<RescaffoldReport, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    rescaffold_active_vault_impl(&config, mode, dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // BC #10: 동일 바이트 → true
    #[test]
    fn bc10_identical_bytes_match() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("a.txt");
        fs::write(&f, b"hello world").unwrap();
        assert!(matches_template(&f, b"hello world"));
    }

    // BC #11: 다른 내용 → false
    #[test]
    fn bc11_different_bytes_do_not_match() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("a.txt");
        fs::write(&f, b"hello").unwrap();
        assert!(!matches_template(&f, b"hello world"));
    }

    // BC #12: 파일 없음 → false
    #[test]
    fn bc12_missing_file_returns_false() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("does-not-exist.txt");
        assert!(!matches_template(&f, b"anything"));
    }

    // BC #13: LF vs CRLF → false (바이트 엄격)
    #[test]
    fn bc13_line_ending_differences_count_as_mismatch() {
        let dir = TempDir::new().unwrap();
        let f = dir.path().join("a.txt");
        fs::write(&f, b"line1\r\nline2").unwrap();
        assert!(!matches_template(&f, b"line1\nline2"));
    }


    // BC #1: 루트 CLAUDE.md, 디스크 없음, 어떤 모드든 → Create
    #[test]
    fn bc1_root_file_missing_creates_in_add_missing() {
        assert_eq!(
            plan_action("CLAUDE.md", RescaffoldMode::AddMissing, false, false),
            FileAction::Create
        );
    }

    #[test]
    fn bc1_root_file_missing_creates_in_force() {
        assert_eq!(
            plan_action("CLAUDE.md", RescaffoldMode::ForceClaude, false, false),
            FileAction::Create
        );
    }

    // BC #2: 루트 CLAUDE.md, 디스크 있음, 어떤 모드든 → Skip (사용자 자산 보호)
    #[test]
    fn bc2_root_file_present_skipped_in_add_missing() {
        // matches 여부에 무관하게 Skip
        assert_eq!(
            plan_action("CLAUDE.md", RescaffoldMode::AddMissing, true, true),
            FileAction::Skip
        );
        assert_eq!(
            plan_action("CLAUDE.md", RescaffoldMode::AddMissing, true, false),
            FileAction::Skip
        );
    }

    #[test]
    fn bc2_root_file_present_skipped_in_force() {
        assert_eq!(
            plan_action("CLAUDE.md", RescaffoldMode::ForceClaude, true, true),
            FileAction::Skip
        );
        assert_eq!(
            plan_action("CLAUDE.md", RescaffoldMode::ForceClaude, true, false),
            FileAction::Skip
        );
    }

    // BC #3: _moc/Home.md 같은 사용자 자산은 ForceClaude에서도 Skip
    #[test]
    fn bc3_moc_file_is_never_overwritten() {
        assert_eq!(
            plan_action("_moc/Home.md", RescaffoldMode::ForceClaude, true, false),
            FileAction::Skip
        );
    }

    // _templates/*도 사용자 자산
    #[test]
    fn templates_are_never_overwritten() {
        assert_eq!(
            plan_action(
                "_templates/tpl-idea.md",
                RescaffoldMode::ForceClaude,
                true,
                false,
            ),
            FileAction::Skip
        );
    }

    // BC #4: .claude/settings.json, 디스크 없음, AddMissing → Create
    #[test]
    fn bc4_claude_missing_creates_in_add_missing() {
        assert_eq!(
            plan_action(
                ".claude/settings.json",
                RescaffoldMode::AddMissing,
                false,
                false,
            ),
            FileAction::Create
        );
    }

    // BC #5: .claude/settings.json, 디스크 없음, ForceClaude → Create
    #[test]
    fn bc5_claude_missing_creates_in_force() {
        assert_eq!(
            plan_action(
                ".claude/settings.json",
                RescaffoldMode::ForceClaude,
                false,
                false,
            ),
            FileAction::Create
        );
    }

    // BC #6: .claude/settings.json, matches=true, AddMissing → Skip
    #[test]
    fn bc6_claude_matching_skipped_in_add_missing() {
        assert_eq!(
            plan_action(
                ".claude/settings.json",
                RescaffoldMode::AddMissing,
                true,
                true,
            ),
            FileAction::Skip
        );
    }

    // BC #7: .claude/settings.json, matches=false, AddMissing → Skip (보수성)
    #[test]
    fn bc7_claude_modified_skipped_in_add_missing() {
        assert_eq!(
            plan_action(
                ".claude/settings.json",
                RescaffoldMode::AddMissing,
                true,
                false,
            ),
            FileAction::Skip
        );
    }

    // BC #8: .claude/settings.json, matches=true, ForceClaude → Skip (이미 최신)
    #[test]
    fn bc8_claude_matching_skipped_in_force() {
        assert_eq!(
            plan_action(
                ".claude/settings.json",
                RescaffoldMode::ForceClaude,
                true,
                true,
            ),
            FileAction::Skip
        );
    }

    // BC #9: .claude/settings.json, matches=false, ForceClaude → Overwrite { user_modified: true }
    #[test]
    fn bc9_claude_modified_overwritten_in_force() {
        assert_eq!(
            plan_action(
                ".claude/settings.json",
                RescaffoldMode::ForceClaude,
                true,
                false,
            ),
            FileAction::Overwrite {
                user_modified: true,
            }
        );
    }

    // 중첩된 .claude/hooks 하위도 동일 분기
    #[test]
    fn claude_hooks_follow_same_branch() {
        assert_eq!(
            plan_action(
                ".claude/hooks/validate-frontmatter.sh",
                RescaffoldMode::ForceClaude,
                true,
                false,
            ),
            FileAction::Overwrite {
                user_modified: true,
            }
        );
    }

    // 모드 JSON 역직렬화 (IPC 경계 보호)
    #[test]
    fn mode_deserializes_from_kebab_case() {
        let add: RescaffoldMode = serde_json::from_str("\"add-missing\"").unwrap();
        let force: RescaffoldMode = serde_json::from_str("\"force-claude\"").unwrap();
        assert_eq!(add, RescaffoldMode::AddMissing);
        assert_eq!(force, RescaffoldMode::ForceClaude);
    }

    // ----- rescaffold_vault 통합 테스트 -----

    fn total_template_files() -> usize {
        vault_files().len()
    }

    /// 이미 완전히 스캐폴드된 볼트를 준비한다 (모든 템플릿 파일이 템플릿과 동일한 상태).
    fn seed_full_vault(dir: &Path) {
        let report =
            rescaffold_vault(dir, RescaffoldMode::AddMissing, false).expect("seed scaffold");
        assert_eq!(report.created.len(), total_template_files());
    }

    // BC #14
    #[test]
    fn bc14_errors_on_missing_root() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope");
        let err = rescaffold_vault(&missing, RescaffoldMode::AddMissing, false).unwrap_err();
        assert!(matches!(err, AppError::VaultNotFound(_)));
    }

    // BC #15
    #[test]
    fn bc15_add_missing_populates_empty_dir() {
        let dir = TempDir::new().unwrap();
        let r = rescaffold_vault(dir.path(), RescaffoldMode::AddMissing, false).unwrap();

        assert_eq!(r.created.len(), total_template_files());
        assert!(r.overwritten.is_empty());
        assert!(r.modified_by_user.is_empty());
        assert_eq!(r.unchanged, 0);
        assert!(!r.dry_run);

        // 실제 디렉토리/파일이 생성되었는지 확인
        for d in VAULT_DIRECTORIES {
            assert!(dir.path().join(d).is_dir(), "dir {} should exist", d);
        }
        assert!(dir.path().join(".claude/settings.json").is_file());
    }

    // BC #16
    #[test]
    fn bc16_noop_on_identical_vault_in_add_missing() {
        let dir = TempDir::new().unwrap();
        seed_full_vault(dir.path());

        let r = rescaffold_vault(dir.path(), RescaffoldMode::AddMissing, false).unwrap();
        assert!(r.created.is_empty());
        assert!(r.overwritten.is_empty());
        assert!(r.modified_by_user.is_empty());
        assert_eq!(r.unchanged, total_template_files());
    }

    // BC #17
    #[test]
    fn bc17_add_missing_preserves_modified_claude_file() {
        let dir = TempDir::new().unwrap();
        seed_full_vault(dir.path());

        // 사용자가 .claude/settings.json 을 수정
        let settings = dir.path().join(".claude/settings.json");
        fs::write(&settings, b"USER_MODIFIED").unwrap();

        let r = rescaffold_vault(dir.path(), RescaffoldMode::AddMissing, false).unwrap();
        assert!(r.created.is_empty());
        assert!(r.overwritten.is_empty());
        assert!(
            r.modified_by_user.is_empty(),
            "add-missing은 경고/덮어쓰기 모두 안 함"
        );
        assert_eq!(r.unchanged, total_template_files());

        // 사용자 수정분은 그대로
        assert_eq!(fs::read(&settings).unwrap(), b"USER_MODIFIED");
    }

    // BC #18
    #[test]
    fn bc18_force_noop_when_all_matching() {
        let dir = TempDir::new().unwrap();
        seed_full_vault(dir.path());

        let r = rescaffold_vault(dir.path(), RescaffoldMode::ForceClaude, false).unwrap();
        assert!(r.overwritten.is_empty());
        assert!(r.modified_by_user.is_empty());
        assert_eq!(r.unchanged, total_template_files());
    }

    // BC #19
    #[test]
    fn bc19_force_overwrites_modified_claude_settings() {
        let dir = TempDir::new().unwrap();
        seed_full_vault(dir.path());

        let settings_rel = ".claude/settings.json";
        let settings = dir.path().join(settings_rel);
        fs::write(&settings, b"USER_MODIFIED").unwrap();

        let r = rescaffold_vault(dir.path(), RescaffoldMode::ForceClaude, false).unwrap();

        assert!(r.overwritten.iter().any(|p| p == settings_rel));
        assert!(r.modified_by_user.iter().any(|p| p == settings_rel));
        // 나머지는 unchanged
        assert_eq!(r.unchanged, total_template_files() - 1);

        // 템플릿 내용으로 복구되었는지
        let template_content = vault_files()
            .into_iter()
            .find(|(p, _)| *p == settings_rel)
            .map(|(_, c)| c)
            .unwrap();
        assert_eq!(fs::read_to_string(&settings).unwrap(), template_content);
    }

    // BC #20 (Unix 한정: hook 재생성 + 실행 권한)
    #[cfg(unix)]
    #[test]
    fn bc20_add_missing_recreates_deleted_hook_with_exec_perm() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        seed_full_vault(dir.path());

        let hook_rel = ".claude/hooks/validate-frontmatter.sh";
        let hook = dir.path().join(hook_rel);
        fs::remove_file(&hook).unwrap();
        assert!(!hook.exists());

        let r = rescaffold_vault(dir.path(), RescaffoldMode::AddMissing, false).unwrap();
        assert!(r.created.iter().any(|p| p == hook_rel));

        let mode = fs::metadata(&hook).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "hook must be executable after recreate");
    }

    // BC #21 (Unix 한정: hook 덮어쓰기 + 실행 권한 재부여)
    #[cfg(unix)]
    #[test]
    fn bc21_force_rewrites_modified_hook_with_exec_perm() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        seed_full_vault(dir.path());

        let hook_rel = ".claude/hooks/validate-frontmatter.sh";
        let hook = dir.path().join(hook_rel);
        // 사용자 수정 + 실행 권한 박탈
        fs::write(&hook, b"#!/bin/sh\necho user-edit").unwrap();
        let mut perms = fs::metadata(&hook).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&hook, perms).unwrap();

        let r = rescaffold_vault(dir.path(), RescaffoldMode::ForceClaude, false).unwrap();
        assert!(r.overwritten.iter().any(|p| p == hook_rel));
        assert!(r.modified_by_user.iter().any(|p| p == hook_rel));

        let mode = fs::metadata(&hook).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "hook must be re-executable after force");
    }

    // BC #22: 사용자 추가 파일은 Report에 등장하지 않음 + 삭제되지 않음
    #[test]
    fn bc22_user_added_files_are_ignored() {
        let dir = TempDir::new().unwrap();
        seed_full_vault(dir.path());

        let custom = dir.path().join(".claude/skills/my-custom/SKILL.md");
        fs::create_dir_all(custom.parent().unwrap()).unwrap();
        fs::write(&custom, b"# my custom skill").unwrap();

        let r = rescaffold_vault(dir.path(), RescaffoldMode::ForceClaude, false).unwrap();
        assert!(r.created.iter().all(|p| !p.contains("my-custom")));
        assert!(r.overwritten.iter().all(|p| !p.contains("my-custom")));
        assert!(r.modified_by_user.iter().all(|p| !p.contains("my-custom")));

        // 사용자 추가 파일은 여전히 존재
        assert!(custom.is_file());
        assert_eq!(fs::read(&custom).unwrap(), b"# my custom skill");
    }

    // BC #23: dry_run=true + ForceClaude → Report 생성되지만 디스크 변경 없음
    #[test]
    fn bc23_dry_run_force_produces_report_without_disk_changes() {
        let dir = TempDir::new().unwrap();
        seed_full_vault(dir.path());

        let settings_rel = ".claude/settings.json";
        let settings = dir.path().join(settings_rel);
        fs::write(&settings, b"USER_MODIFIED").unwrap();

        let r = rescaffold_vault(dir.path(), RescaffoldMode::ForceClaude, true).unwrap();
        assert!(r.dry_run);
        assert!(r.overwritten.iter().any(|p| p == settings_rel));
        assert!(r.modified_by_user.iter().any(|p| p == settings_rel));

        // 실제 디스크는 수정된 그대로
        assert_eq!(fs::read(&settings).unwrap(), b"USER_MODIFIED");
    }

    // BC #24: dry_run=true + AddMissing + 파일 누락 → Report 만들지만 파일 생성 X
    #[test]
    fn bc24_dry_run_add_missing_produces_report_without_disk_changes() {
        let dir = TempDir::new().unwrap();

        let r = rescaffold_vault(dir.path(), RescaffoldMode::AddMissing, true).unwrap();
        assert!(r.dry_run);
        assert_eq!(r.created.len(), total_template_files());

        // 실제 파일은 생성되지 않음
        assert!(!dir.path().join(".claude/settings.json").exists());
        assert!(!dir.path().join("_moc/Home.md").exists());
    }

    // BC #25: vault_path == None → VaultNotConfigured
    #[test]
    fn bc25_ipc_errors_when_vault_not_configured() {
        let config = AppConfig::default();
        assert!(config.vault_path.is_none());

        let err = rescaffold_active_vault_impl(&config, RescaffoldMode::AddMissing, true)
            .unwrap_err();
        assert!(matches!(err, AppError::VaultNotConfigured));
    }

    // BC #26/#27: vault_path가 설정된 경우 rescaffold_vault에 그대로 위임
    #[test]
    fn bc26_ipc_delegates_to_rescaffold_vault() {
        let dir = TempDir::new().unwrap();
        let config = AppConfig {
            vault_path: Some(PathBuf::from(dir.path())),
            ..Default::default()
        };

        let r =
            rescaffold_active_vault_impl(&config, RescaffoldMode::AddMissing, true).unwrap();
        assert!(r.dry_run);
        assert_eq!(r.created.len(), total_template_files());
        // dry-run이므로 파일은 생성되지 않음
        assert!(!dir.path().join(".claude/settings.json").exists());
    }

    // Invariant: created + overwritten + unchanged == 템플릿 파일 총 수
    #[test]
    fn invariant_report_covers_all_templates() {
        let dir = TempDir::new().unwrap();
        seed_full_vault(dir.path());

        let settings = dir.path().join(".claude/settings.json");
        fs::write(&settings, b"USER_MODIFIED").unwrap();

        for mode in [RescaffoldMode::AddMissing, RescaffoldMode::ForceClaude] {
            for dry in [true, false] {
                // 매 이터레이션마다 다시 seed해서 독립 검증
                let subdir = TempDir::new().unwrap();
                seed_full_vault(subdir.path());
                fs::write(subdir.path().join(".claude/settings.json"), b"USER").unwrap();

                let r = rescaffold_vault(subdir.path(), mode, dry).unwrap();
                assert_eq!(
                    r.created.len() + r.overwritten.len() + r.unchanged,
                    total_template_files(),
                    "mode={:?} dry={} invariant violation",
                    mode,
                    dry
                );
            }
        }
    }
}
