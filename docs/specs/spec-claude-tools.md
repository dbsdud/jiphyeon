# Spec: claude-tools (v0.5 볼트 Claude 도구 뷰)

## 전제

- 앱은 **뷰어** — 스킬/훅을 읽기만 한다. 실행/편집 없음.
- v0.5 범위: SKILL.md의 `name`/`description`, settings.json의 훅 매핑, 루트 CLAUDE.md 존재 여부.
- 파싱 실패는 **조용히 스킵하지 않는다** — `skill_warnings`/`hooks_error` 필드로 UI에 노출해 사용자가 고칠 수 있게 한다.
- 전체 결과 반환은 가능한 한 유지한다 (부분 실패가 전체 요청을 깨지 않음).
- 볼트 미연결 시에는 `VaultNotConfigured` 에러 반환 (기존 명령 일관성).

## Public Interface

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ClaudeSkill {
    pub name: String,
    pub description: String,
    /// 볼트 기준 상대경로 (예: ".claude/skills/vault-audit/SKILL.md")
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeHook {
    pub event: String,               // "SessionStart" | "PreToolUse" | "PostToolUse" | "Stop" | "UserPromptSubmit" 등
    pub matcher: Option<String>,     // 예: "Write|Edit"
    pub command: String,             // settings.json의 원본 command
    /// command가 `bash <path> [args...]` 형태일 때 추출한 상대경로
    pub script_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillWarning {
    pub path: String,    // 볼트 기준 상대경로
    pub reason: String,  // 사용자에게 보여줄 한국어 사유
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ClaudeTools {
    pub claude_md: Option<String>,            // 볼트 루트 "CLAUDE.md" 존재 시 Some("CLAUDE.md")
    pub skills: Vec<ClaudeSkill>,
    pub skill_warnings: Vec<SkillWarning>,    // 파싱 실패한 SKILL.md 목록 (UI에서 경고)
    pub hooks: Vec<ClaudeHook>,
    pub hooks_error: Option<String>,          // settings.json 파싱/읽기 전체 실패 시 메시지
}

#[tauri::command]
pub fn get_claude_tools(config_state: State<'_, ConfigState>) -> Result<ClaudeTools, AppError>

// --- 순수 파싱 헬퍼 (유닛 테스트 대상) ---

/// SKILL.md 내용을 파싱.
/// 성공: Ok(ClaudeSkill). 실패: Err(사용자 노출용 한국어 사유).
pub fn parse_skill_md(content: &str, rel_path: &str) -> Result<ClaudeSkill, String>

/// settings.json 내용을 파싱.
/// 성공: Ok(Vec<ClaudeHook>). 실패: Err(사용자 노출용 한국어 사유).
pub fn parse_hooks_json(content: &str) -> Result<Vec<ClaudeHook>, String>

/// `bash <path> [args...]` 패턴이면 path 반환, 아니면 None.
pub fn extract_bash_script_path(command: &str) -> Option<String>
```

## Invariants

- `get_claude_tools`는 파싱 단계에서 panic하지 않는다.
- 스킬 디렉토리 일부만 유효해도 유효한 것들은 반환된다.
- `claude_md`는 **볼트 루트의 `CLAUDE.md`** 존재 여부로만 결정 (하위 폴더에 있는 CLAUDE.md는 무시).
- `ClaudeHook.command`는 원본 그대로 보존 — UI에서 명시적으로 표시.
- `script_path`는 있으면 볼트 기준 **상대경로**. 절대경로가 오면 그대로 반환 (검증 X).

## Behavior Contract — parse_skill_md

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 정상 SKILL.md (`name: x`, `description: y`) | `parse_skill_md` | `Ok(ClaudeSkill { name: "x", description: "y", path: rel })` |
| 2 | frontmatter 없음 (마크다운만) | `parse_skill_md` | `Err("frontmatter(--- ... ---) 블록이 없음")` |
| 3 | `name`만 있고 `description` 없음 | `parse_skill_md` | `Err("description 필드 누락")` |
| 4 | `description`만 있고 `name` 없음 | `parse_skill_md` | `Err("name 필드 누락")` |
| 5 | frontmatter YAML 파싱 실패 | `parse_skill_md` | `Err("YAML 파싱 실패: ...")` |

## Behavior Contract — extract_bash_script_path

| # | Given | When | Then |
|---|-------|------|------|
| 6 | `"bash .claude/hooks/foo.sh"` | `extract_bash_script_path` | `Some(".claude/hooks/foo.sh")` |
| 7 | `"bash .claude/hooks/foo.sh arg1 arg2"` | `extract_bash_script_path` | `Some(".claude/hooks/foo.sh")` |
| 8 | `"  bash  .claude/hooks/foo.sh  "` (공백) | `extract_bash_script_path` | `Some(".claude/hooks/foo.sh")` |
| 9 | `"echo hello"` | `extract_bash_script_path` | `None` |
| 10 | `""` (빈 문자열) | `extract_bash_script_path` | `None` |
| 11 | `"bash"` (경로 없음) | `extract_bash_script_path` | `None` |

## Behavior Contract — parse_hooks_json

| # | Given | When | Then |
|---|-------|------|------|
| 12 | 정상 settings.json (SessionStart 1개, PreToolUse matcher Write\|Edit 2개) | `parse_hooks_json` | `Ok(3개 훅 플랫 리스트)`, 각 `event` 올바름 |
| 13 | matcher 없는 이벤트 | `parse_hooks_json` | `matcher: None` 유지 |
| 14 | `bash .claude/hooks/x.sh` 커맨드 | `parse_hooks_json` | 해당 훅의 `script_path == Some(".claude/hooks/x.sh")` |
| 15 | 비-bash 커맨드 (`git pull --rebase ...`) | `parse_hooks_json` | `script_path: None`, `command`은 원본 유지 |
| 16 | 깨진 JSON | `parse_hooks_json` | `Err("JSON 파싱 실패: ...")` |
| 17 | `hooks` 키 없음 | `parse_hooks_json` | `Ok(빈 Vec)` |

## Behavior Contract — get_claude_tools

| # | Given | When | Then |
|---|-------|------|------|
| 18 | vault_path == None | `get_claude_tools` | `AppError::VaultNotConfigured` |
| 19 | 볼트 루트에 `.claude/` 자체가 없음 | `get_claude_tools` | `skills: []`, `hooks: []`, `claude_md: ?` (루트 CLAUDE.md 유무에 따름) |
| 20 | 볼트 루트에 `CLAUDE.md` 있음 | `get_claude_tools` | `claude_md == Some("CLAUDE.md")` |
| 21 | 루트 `CLAUDE.md` 없음 | `get_claude_tools` | `claude_md == None` |
| 22 | `.claude/skills/` 하위에 유효 SKILL.md 3개 | `get_claude_tools` | `skills.len() == 3`, `skill_warnings: []` |
| 23 | `.claude/skills/` 하위에 유효 2개 + 깨진 1개 | `get_claude_tools` | `skills.len() == 2`, `skill_warnings.len() == 1`, warning에 path + 사유 포함 |
| 24 | `.claude/settings.json`이 유효 | `get_claude_tools` | `hooks`는 플랫 리스트, `hooks_error: None` |
| 25 | `.claude/settings.json`이 깨짐 | `get_claude_tools` | `hooks: []`, `hooks_error: Some("JSON 파싱 실패: ...")` |
| 26 | `.claude/settings.json` 파일 없음 | `get_claude_tools` | `hooks: []`, `hooks_error: None` |
| 27 | `.claude/settings.json` 읽기 권한 없음 | `get_claude_tools` | `hooks: []`, `hooks_error: Some("파일 읽기 실패: ...")` |

## Edge Cases

- 스킬 디렉토리 내에 SKILL.md 외 다른 파일(readme 등)이 있어도 무시
- 하위 중첩 스킬(`.claude/skills/a/b/SKILL.md`) — v0.5 비범위 (최상위 1레벨만 스캔)
- `claude_md`가 존재하지만 읽기 권한 없는 경우 — `Some("CLAUDE.md")` 반환 (렌더링 시 실패)
- 스킬/훅 정렬: 스킬은 `name` 알파벳 순, 훅은 `event` + 원본 순서 유지. `skill_warnings`는 발견 순서.

## UI 노출 (B2)

- 스킬 섹션 하단에 `skill_warnings`가 있으면 접힌 경고 패널 ("⚠️ N개의 SKILL.md를 읽을 수 없음") — 펼치면 path/reason 목록
- 훅 섹션 상단에 `hooks_error`가 있으면 경고 배너 노출 (원본 에러 메시지 포함)
- 파싱된 훅 리스트는 오류와 별개로 **정상 렌더** (부분 장애를 전체로 확대하지 않음)

## Dependencies

- 기존 `config::ConfigState`, `error::AppError`
- 기존 `vault::parser::extract_frontmatter` 재사용 가능 여부 확인 → 기존은 `Frontmatter`(노트 타입) 반환이라 부적합. SKILL 전용 YAML 파싱 필요 (serde_yaml_ng + Deserialize 정의).
- `serde_json::Value` — 훅 settings.json 파싱 (구조 유연성)
- Mock boundary: 파일 시스템 (TempDir로 통합 테스트)

## Mock boundary

- `parse_skill_md` / `parse_hooks_json` / `extract_bash_script_path`: 순수 함수, 문자열 입력 → 쉽게 테스트
- `get_claude_tools`: TempDir 기반 통합 테스트 (가짜 볼트 + `.claude/` 구조 만들어서 전체 플로우 검증)
