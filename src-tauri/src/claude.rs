//! 볼트의 Claude 도구(스킬/훅/CLAUDE.md) 수집 + 파싱
//!
//! 순수 파서(parse_skill_md, parse_hooks_json, extract_bash_script_path)는
//! 문자열 입력 → Result 반환. 실패 사유는 Err(String)으로 표면화되어
//! UI에서 경고로 노출된다.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ClaudeSkill {
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ClaudeHook {
    pub event: String,
    pub matcher: Option<String>,
    pub command: String,
    pub script_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillWarning {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ClaudeTools {
    pub claude_md: Option<String>,
    pub skills: Vec<ClaudeSkill>,
    pub skill_warnings: Vec<SkillWarning>,
    pub hooks: Vec<ClaudeHook>,
    pub hooks_error: Option<String>,
}

#[derive(Deserialize)]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
}

fn extract_yaml_block(content: &str) -> Option<&str> {
    if !content.starts_with("---") {
        return None;
    }
    let rest = &content[3..];
    let end = rest.find("\n---")?;
    Some(rest[..end].trim())
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn parse_skill_md(content: &str, rel_path: &str) -> Result<ClaudeSkill, String> {
    let yaml = extract_yaml_block(content)
        .ok_or_else(|| "frontmatter(--- ... ---) 블록이 없음".to_string())?;

    let fm: SkillFrontmatter = serde_yaml_ng::from_str(yaml)
        .map_err(|e| format!("YAML 파싱 실패: {e}"))?;

    let name = non_empty(fm.name).ok_or_else(|| "name 필드 누락".to_string())?;
    let description =
        non_empty(fm.description).ok_or_else(|| "description 필드 누락".to_string())?;

    Ok(ClaudeSkill {
        name,
        description,
        path: rel_path.to_string(),
    })
}

/// `bash <path> [args...]` 패턴에서 `<path>`만 추출.
pub fn extract_bash_script_path(command: &str) -> Option<String> {
    let mut tokens = command.split_whitespace();
    let first = tokens.next()?;
    if first != "bash" {
        return None;
    }
    let path = tokens.next()?;
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

pub fn parse_hooks_json(content: &str) -> Result<Vec<ClaudeHook>, String> {
    let value: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| format!("JSON 파싱 실패: {e}"))?;

    let mut result = Vec::new();
    let Some(hooks_obj) = value.get("hooks").and_then(|v| v.as_object()) else {
        return Ok(result);
    };

    for (event, event_value) in hooks_obj {
        let Some(groups) = event_value.as_array() else {
            continue;
        };
        for group in groups {
            let matcher = group
                .get("matcher")
                .and_then(|v| v.as_str())
                .map(String::from);
            let Some(group_hooks) = group.get("hooks").and_then(|v| v.as_array()) else {
                continue;
            };
            for hook in group_hooks {
                let Some(command) = hook.get("command").and_then(|v| v.as_str()) else {
                    continue;
                };
                let script_path = extract_bash_script_path(command);
                result.push(ClaudeHook {
                    event: event.clone(),
                    matcher: matcher.clone(),
                    command: command.to_string(),
                    script_path,
                });
            }
        }
    }

    Ok(result)
}

/// 볼트 루트에서 스킬/훅/CLAUDE.md를 수집한다.
/// 부분 실패는 `skill_warnings` / `hooks_error`에 누적되고 전체 결과는 항상 반환한다.
pub fn collect_claude_tools(vault_root: &Path) -> ClaudeTools {
    let mut tools = ClaudeTools::default();

    if vault_root.join("CLAUDE.md").is_file() {
        tools.claude_md = Some("CLAUDE.md".to_string());
    }

    collect_skills(vault_root, &mut tools);
    collect_hooks(vault_root, &mut tools);

    tools.skills.sort_by(|a, b| a.name.cmp(&b.name));
    tools
}

fn collect_skills(vault_root: &Path, tools: &mut ClaudeTools) {
    let skills_dir = vault_root.join(".claude/skills");
    let Ok(entries) = fs::read_dir(&skills_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let skill_file = dir.join("SKILL.md");
        if !skill_file.is_file() {
            continue;
        }
        let dir_name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let rel_path = format!(".claude/skills/{}/SKILL.md", dir_name);

        match fs::read_to_string(&skill_file) {
            Ok(content) => match parse_skill_md(&content, &rel_path) {
                Ok(skill) => tools.skills.push(skill),
                Err(reason) => tools.skill_warnings.push(SkillWarning {
                    path: rel_path,
                    reason,
                }),
            },
            Err(e) => tools.skill_warnings.push(SkillWarning {
                path: rel_path,
                reason: format!("파일 읽기 실패: {e}"),
            }),
        }
    }
}

fn collect_hooks(vault_root: &Path, tools: &mut ClaudeTools) {
    let settings_path = vault_root.join(".claude/settings.json");
    if !settings_path.is_file() {
        return;
    }
    match fs::read_to_string(&settings_path) {
        Ok(content) => match parse_hooks_json(&content) {
            Ok(hooks) => tools.hooks = hooks,
            Err(reason) => tools.hooks_error = Some(reason),
        },
        Err(e) => tools.hooks_error = Some(format!("파일 읽기 실패: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- parse_skill_md ---

    // BC #1: 정상
    #[test]
    fn parse_skill_md_ok() {
        let content = "---\nname: vault-audit\ndescription: 볼트 감사\n---\n\n# content";
        let s = parse_skill_md(content, ".claude/skills/vault-audit/SKILL.md").unwrap();
        assert_eq!(s.name, "vault-audit");
        assert_eq!(s.description, "볼트 감사");
        assert_eq!(s.path, ".claude/skills/vault-audit/SKILL.md");
    }

    // BC #2: frontmatter 없음
    #[test]
    fn parse_skill_md_without_frontmatter() {
        let err = parse_skill_md("# just markdown", "a").unwrap_err();
        assert!(err.contains("frontmatter"));
    }

    // BC #3: description 누락
    #[test]
    fn parse_skill_md_missing_description() {
        let content = "---\nname: x\n---\n";
        let err = parse_skill_md(content, "a").unwrap_err();
        assert!(err.contains("description"));
    }

    // BC #4: name 누락
    #[test]
    fn parse_skill_md_missing_name() {
        let content = "---\ndescription: y\n---\n";
        let err = parse_skill_md(content, "a").unwrap_err();
        assert!(err.contains("name"));
    }

    // BC #5: YAML 파싱 실패
    #[test]
    fn parse_skill_md_invalid_yaml() {
        let content = "---\nname: [unclosed\n---\n";
        let err = parse_skill_md(content, "a").unwrap_err();
        assert!(err.contains("YAML"));
    }

    // name이 빈 문자열이면 누락으로 취급
    #[test]
    fn parse_skill_md_empty_name_is_missing() {
        let content = "---\nname: \"\"\ndescription: y\n---\n";
        let err = parse_skill_md(content, "a").unwrap_err();
        assert!(err.contains("name"));
    }

    // --- extract_bash_script_path ---

    // BC #6
    #[test]
    fn extract_bash_basic() {
        assert_eq!(
            extract_bash_script_path("bash .claude/hooks/foo.sh"),
            Some(".claude/hooks/foo.sh".to_string())
        );
    }

    // BC #7
    #[test]
    fn extract_bash_with_args() {
        assert_eq!(
            extract_bash_script_path("bash .claude/hooks/foo.sh arg1 arg2"),
            Some(".claude/hooks/foo.sh".to_string())
        );
    }

    // BC #8
    #[test]
    fn extract_bash_with_whitespace() {
        assert_eq!(
            extract_bash_script_path("  bash  .claude/hooks/foo.sh  "),
            Some(".claude/hooks/foo.sh".to_string())
        );
    }

    // BC #9
    #[test]
    fn extract_bash_not_bash_command() {
        assert_eq!(extract_bash_script_path("echo hello"), None);
    }

    // BC #10
    #[test]
    fn extract_bash_empty() {
        assert_eq!(extract_bash_script_path(""), None);
    }

    // BC #11
    #[test]
    fn extract_bash_only_bash() {
        assert_eq!(extract_bash_script_path("bash"), None);
    }

    // --- parse_hooks_json ---

    // BC #12, #13, #14, #15
    #[test]
    fn parse_hooks_json_flattens_structure() {
        let content = r#"{
          "hooks": {
            "SessionStart": [
              { "hooks": [{ "type": "command", "command": "git pull --rebase" }] }
            ],
            "PreToolUse": [
              {
                "matcher": "Write|Edit",
                "hooks": [
                  { "type": "command", "command": "bash .claude/hooks/foo.sh" },
                  { "type": "command", "command": "bash .claude/hooks/bar.sh" }
                ]
              }
            ]
          }
        }"#;

        let hooks = parse_hooks_json(content).unwrap();
        assert_eq!(hooks.len(), 3);

        let session = hooks.iter().find(|h| h.event == "SessionStart").unwrap();
        assert_eq!(session.matcher, None);
        assert_eq!(session.command, "git pull --rebase");
        assert_eq!(session.script_path, None);

        let pre: Vec<_> = hooks.iter().filter(|h| h.event == "PreToolUse").collect();
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0].matcher.as_deref(), Some("Write|Edit"));
        assert_eq!(pre[0].script_path.as_deref(), Some(".claude/hooks/foo.sh"));
    }

    // BC #16
    #[test]
    fn parse_hooks_json_invalid() {
        let err = parse_hooks_json("{not json").unwrap_err();
        assert!(err.contains("JSON"));
    }

    // BC #17
    #[test]
    fn parse_hooks_json_without_hooks_key() {
        let hooks = parse_hooks_json(r#"{"other": 1}"#).unwrap();
        assert!(hooks.is_empty());
    }

    // --- collect_claude_tools (integration) ---

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn valid_skill(name: &str, desc: &str) -> String {
        format!("---\nname: {name}\ndescription: {desc}\n---\n\n# body")
    }

    // BC #19: .claude/ 없음
    #[test]
    fn collect_empty_vault_returns_empty_tools() {
        let dir = TempDir::new().unwrap();
        let tools = collect_claude_tools(dir.path());
        assert!(tools.skills.is_empty());
        assert!(tools.hooks.is_empty());
        assert!(tools.skill_warnings.is_empty());
        assert!(tools.hooks_error.is_none());
        assert!(tools.claude_md.is_none());
    }

    // BC #20 + #21
    #[test]
    fn collect_detects_claude_md_presence() {
        let dir = TempDir::new().unwrap();

        let tools_missing = collect_claude_tools(dir.path());
        assert_eq!(tools_missing.claude_md, None);

        write(&dir.path().join("CLAUDE.md"), "# Vault");
        let tools_present = collect_claude_tools(dir.path());
        assert_eq!(tools_present.claude_md.as_deref(), Some("CLAUDE.md"));
    }

    // BC #22: 유효 3개
    #[test]
    fn collect_three_valid_skills() {
        let dir = TempDir::new().unwrap();
        write(
            &dir.path().join(".claude/skills/a/SKILL.md"),
            &valid_skill("a-skill", "desc a"),
        );
        write(
            &dir.path().join(".claude/skills/b/SKILL.md"),
            &valid_skill("b-skill", "desc b"),
        );
        write(
            &dir.path().join(".claude/skills/c/SKILL.md"),
            &valid_skill("c-skill", "desc c"),
        );

        let tools = collect_claude_tools(dir.path());
        assert_eq!(tools.skills.len(), 3);
        assert!(tools.skill_warnings.is_empty());
        // 알파벳 정렬
        assert_eq!(tools.skills[0].name, "a-skill");
        assert_eq!(tools.skills[1].name, "b-skill");
        assert_eq!(tools.skills[2].name, "c-skill");
    }

    // BC #23: 유효 2개 + 깨진 1개 → skill_warnings에 1개
    #[test]
    fn collect_partial_skills_emits_warnings() {
        let dir = TempDir::new().unwrap();
        write(
            &dir.path().join(".claude/skills/good/SKILL.md"),
            &valid_skill("good", "ok"),
        );
        write(
            &dir.path().join(".claude/skills/also-good/SKILL.md"),
            &valid_skill("also", "ok"),
        );
        write(
            &dir.path().join(".claude/skills/broken/SKILL.md"),
            "# no frontmatter here",
        );

        let tools = collect_claude_tools(dir.path());
        assert_eq!(tools.skills.len(), 2);
        assert_eq!(tools.skill_warnings.len(), 1);
        assert_eq!(
            tools.skill_warnings[0].path,
            ".claude/skills/broken/SKILL.md"
        );
        assert!(tools.skill_warnings[0].reason.contains("frontmatter"));
    }

    // BC #24
    #[test]
    fn collect_valid_settings_returns_hooks_without_error() {
        let dir = TempDir::new().unwrap();
        let settings = r#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"git pull"}]}]}}"#;
        write(&dir.path().join(".claude/settings.json"), settings);

        let tools = collect_claude_tools(dir.path());
        assert_eq!(tools.hooks.len(), 1);
        assert!(tools.hooks_error.is_none());
    }

    // BC #25: 깨진 settings.json → hooks_error
    #[test]
    fn collect_broken_settings_surfaces_error() {
        let dir = TempDir::new().unwrap();
        write(&dir.path().join(".claude/settings.json"), "{broken json");

        let tools = collect_claude_tools(dir.path());
        assert!(tools.hooks.is_empty());
        assert!(tools.hooks_error.as_deref().unwrap().contains("JSON"));
    }

    // BC #26: settings.json 파일 없음
    #[test]
    fn collect_no_settings_file() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();

        let tools = collect_claude_tools(dir.path());
        assert!(tools.hooks.is_empty());
        assert!(tools.hooks_error.is_none());
    }
}
