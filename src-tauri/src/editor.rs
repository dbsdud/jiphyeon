//! 에디터 감지 + editor_command 해석
//!
//! - `resolve_editor`: 순수 함수. editor_command 문자열을 `ResolvedEditor`로 해석
//! - `detect_editors`: 플랫폼별 설치 에디터 감지 (macOS + Unix, Windows는 베스트 에포트)

use std::path::Path;

use serde::Serialize;

/// editor_command + 대상 경로 → 실행 방식 분기
#[derive(Debug, PartialEq, Eq)]
pub enum ResolvedEditor {
    /// 실행파일 + 인자
    Command { program: String, args: Vec<String> },
    /// URL 스킴 (opener 플러그인으로 열기)
    Url(String),
}

/// 감지된 에디터 후보
#[derive(Debug, Clone, Serialize)]
pub struct DetectedEditor {
    pub id: String,
    pub label: String,
    pub command: String,
}

const PATH_PLACEHOLDER: &str = "{path}";

/// editor_command를 해석한다.
/// - `://` 포함 → Url (필요 시 `{path}` 치환, URL 인코딩)
/// - 그 외 → Command (플레이스홀더 있으면 치환, 없으면 인자 append)
pub fn resolve_editor(editor_command: &str, target: &Path) -> ResolvedEditor {
    let target_str = target.to_string_lossy();

    if editor_command.contains("://") {
        let encoded = urlencoding::encode(&target_str).into_owned();
        let url = editor_command.replace(PATH_PLACEHOLDER, &encoded);
        ResolvedEditor::Url(url)
    } else {
        let trimmed = editor_command.trim();
        let mut tokens: Vec<String> = trimmed.split_whitespace().map(String::from).collect();

        if tokens.is_empty() {
            return ResolvedEditor::Command {
                program: String::new(),
                args: vec![target_str.into_owned()],
            };
        }

        let program = tokens.remove(0);
        let has_placeholder = tokens.iter().any(|t| t.contains(PATH_PLACEHOLDER));

        let args: Vec<String> = if has_placeholder {
            tokens
                .into_iter()
                .map(|t| t.replace(PATH_PLACEHOLDER, &target_str))
                .collect()
        } else {
            // 플레이스홀더 없으면 경로를 마지막 인자로 append (기존 동작 호환)
            tokens.push(target_str.into_owned());
            tokens
        };

        ResolvedEditor::Command { program, args }
    }
}

/// 시스템에서 설치된 에디터 후보를 감지한다.
/// 감지 실패는 panic하지 않고 해당 항목을 목록에서 제외.
pub fn detect_editors() -> Vec<DetectedEditor> {
    let mut results = Vec::new();

    for candidate in CANDIDATES {
        if let Some(editor) = candidate.detect() {
            results.push(editor);
        }
    }

    results
}

struct Candidate {
    id: &'static str,
    label: &'static str,
    /// macOS 앱 번들 이름 (없으면 None)
    mac_app: Option<&'static str>,
    /// `which`로 찾을 실행파일 이름 (없으면 None)
    bin: Option<&'static str>,
    /// URL 스킴 템플릿 (설정 시 Command 감지 대신 Url 모드로 저장됨)
    url_template: Option<&'static str>,
}

const CANDIDATES: &[Candidate] = &[
    Candidate {
        id: "vscode",
        label: "VS Code",
        mac_app: Some("Visual Studio Code"),
        bin: Some("code"),
        url_template: None,
    },
    Candidate {
        id: "cursor",
        label: "Cursor",
        mac_app: Some("Cursor"),
        bin: Some("cursor"),
        url_template: None,
    },
    Candidate {
        id: "zed",
        label: "Zed",
        mac_app: Some("Zed"),
        bin: Some("zed"),
        url_template: None,
    },
    Candidate {
        id: "sublime",
        label: "Sublime Text",
        mac_app: Some("Sublime Text"),
        bin: Some("subl"),
        url_template: None,
    },
    Candidate {
        id: "obsidian",
        label: "Obsidian",
        mac_app: Some("Obsidian"),
        bin: None,
        url_template: Some("obsidian://open?path={path}"),
    },
];

impl Candidate {
    fn detect(&self) -> Option<DetectedEditor> {
        // URL 모드 에디터(Obsidian)는 앱 존재만으로 감지
        if let Some(url_template) = self.url_template {
            if self.mac_app_installed() {
                return Some(DetectedEditor {
                    id: self.id.to_string(),
                    label: self.label.to_string(),
                    command: url_template.to_string(),
                });
            }
            return None;
        }

        // Command 모드: macOS 앱 경로 → CLI wrapper 경로 → `which` 순
        if let Some(cli_path) = self.mac_cli_path() {
            return Some(self.as_detected(cli_path));
        }
        if let Some(bin_path) = self.which_bin() {
            return Some(self.as_detected(bin_path));
        }
        None
    }

    fn mac_app_installed(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            if let Some(app) = self.mac_app {
                return Path::new(&format!("/Applications/{app}.app")).exists();
            }
        }
        false
    }

    /// macOS 앱이 제공하는 CLI 런처 경로 (예: VS Code의 `code` wrapper)
    fn mac_cli_path(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            if let (Some(app), Some(bin)) = (self.mac_app, self.bin) {
                let path = format!(
                    "/Applications/{app}.app/Contents/Resources/app/bin/{bin}"
                );
                if Path::new(&path).exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    fn which_bin(&self) -> Option<String> {
        let bin = self.bin?;
        let output = std::process::Command::new("which").arg(bin).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let path = String::from_utf8(output.stdout).ok()?.trim().to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    fn as_detected(&self, command: String) -> DetectedEditor {
        DetectedEditor {
            id: self.id.to_string(),
            label: self.label.to_string(),
            command,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // --- resolve_editor: URL 모드 ---

    // BC #6: URL 템플릿 + {path} → URL 인코딩 후 치환
    #[test]
    fn resolve_url_substitutes_encoded_path() {
        let r = resolve_editor("obsidian://open?path={path}", &PathBuf::from("/a/b.md"));
        assert_eq!(r, ResolvedEditor::Url("obsidian://open?path=%2Fa%2Fb.md".to_string()));
    }

    // BC #9: URL에 {path} 없으면 원문 그대로
    #[test]
    fn resolve_url_without_placeholder_passes_through() {
        let r = resolve_editor("obsidian://vault", &PathBuf::from("/a/b.md"));
        assert_eq!(r, ResolvedEditor::Url("obsidian://vault".to_string()));
    }

    // --- resolve_editor: Command 모드 ---

    // BC #7: 플레이스홀더가 인자 토큰에 있으면 치환
    #[test]
    fn resolve_command_substitutes_placeholder_arg() {
        let r = resolve_editor("/usr/local/bin/code {path}", &PathBuf::from("/a/b.md"));
        assert_eq!(
            r,
            ResolvedEditor::Command {
                program: "/usr/local/bin/code".to_string(),
                args: vec!["/a/b.md".to_string()],
            }
        );
    }

    // BC #8: 플레이스홀더 없으면 경로를 인자로 append (기존 동작 호환)
    #[test]
    fn resolve_command_appends_path_when_no_placeholder() {
        let r = resolve_editor("/usr/local/bin/code", &PathBuf::from("/a/b.md"));
        assert_eq!(
            r,
            ResolvedEditor::Command {
                program: "/usr/local/bin/code".to_string(),
                args: vec!["/a/b.md".to_string()],
            }
        );
    }

    // BC #10: 빈/공백 문자열 → 빈 program + 경로 인자 (spawn 단계에서 에러)
    #[test]
    fn resolve_command_empty_yields_empty_program() {
        let r = resolve_editor("   ", &PathBuf::from("/a/b.md"));
        assert_eq!(
            r,
            ResolvedEditor::Command {
                program: String::new(),
                args: vec!["/a/b.md".to_string()],
            }
        );
    }

    // Edge: 여러 개의 {path} 전부 치환
    #[test]
    fn resolve_url_replaces_multiple_placeholders() {
        let r = resolve_editor(
            "myeditor://open?src={path}&alt={path}",
            &PathBuf::from("/a/b.md"),
        );
        assert_eq!(
            r,
            ResolvedEditor::Url("myeditor://open?src=%2Fa%2Fb.md&alt=%2Fa%2Fb.md".to_string())
        );
    }

    // Edge: 인자 중간에 플레이스홀더 + 뒤에 추가 인자
    #[test]
    fn resolve_command_with_mixed_args() {
        let r = resolve_editor("mycode --file {path} --flag", &PathBuf::from("/x.md"));
        assert_eq!(
            r,
            ResolvedEditor::Command {
                program: "mycode".to_string(),
                args: vec!["--file".to_string(), "/x.md".to_string(), "--flag".to_string()],
            }
        );
    }

    // --- detect_editors: smoke ---

    // BC #1: 플랫폼 무관 panic 없음
    #[test]
    fn detect_editors_does_not_panic() {
        let _list = detect_editors();
    }

    // BC #5: 감지된 항목이 있다면 모든 필드 채워짐
    #[test]
    fn detect_editors_returns_well_formed_entries() {
        let list = detect_editors();
        for e in list {
            assert!(!e.id.is_empty());
            assert!(!e.label.is_empty());
            assert!(!e.command.is_empty());
        }
    }
}
