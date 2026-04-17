# Spec: settings (v0.5 에디터 유연성 + 설정 편집)

## 전제

- `editor_command`는 **단일 문자열**로 커맨드와 URL 스킴을 모두 표현한다.
  - `://` 포함 → URL 스킴 (예: `obsidian://open?path={path}`)
  - 미포함 → 실행파일 경로 + 선택적 인자 템플릿 (예: `/usr/local/bin/code {path}`)
- `{path}` 플레이스홀더는 외부 에디터로 열 대상 파일의 **절대경로**로 치환된다.
  - URL 모드: URL 인코딩 후 치환
  - Command 모드: 그대로 치환. 플레이스홀더가 없으면 경로를 마지막 인자로 append (현재 동작과 호환)

## Public Interface

```rust
/// 시스템에서 감지된 에디터 후보
#[derive(Debug, Clone, Serialize)]
pub struct DetectedEditor {
    pub id: String,        // "vscode" | "cursor" | "zed" | "sublime" | "obsidian"
    pub label: String,     // 표시용 이름 (예: "VS Code", "Obsidian")
    pub command: String,   // editor_command 필드에 저장될 문자열
}

/// 시스템에 설치된 에디터 후보 감지
#[tauri::command]
pub fn detect_editors() -> Vec<DetectedEditor>

/// 편집 가능한 필드만 포함하는 부분 업데이트 patch
/// `vault_path`는 타입 레벨에서 제외 (온보딩 전용 커맨드로만 변경)
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfigPatch {
    pub editor_command: Option<String>,
    pub exclude_dirs: Option<Vec<String>>,
    pub recent_notes_limit: Option<usize>,
    pub global_shortcut: Option<String>,
    pub quick_note_folder: Option<String>,
}

/// 현재 설정 스냅샷 반환 (clone)
#[tauri::command]
pub fn get_config(config_state: State<'_, ConfigState>) -> Result<AppConfig, AppError>

/// Patch 머지 후 영속화. 업데이트된 AppConfig 반환.
#[tauri::command]
pub fn update_config(
    config_state: State<'_, ConfigState>,
    app_handle: AppHandle,
    patch: AppConfigPatch,
) -> Result<AppConfig, AppError>

/// editor_command + 대상 경로 → 실행 방식 분기
#[derive(Debug, PartialEq, Eq)]
pub enum ResolvedEditor {
    Command { program: String, args: Vec<String> }, // 실행파일 경로 + 인자
    Url(String),                                    // 최종 URL (이미 {path} 치환 완료)
}

pub fn resolve_editor(editor_command: &str, target: &Path) -> ResolvedEditor
```

## Invariants

- `update_config(patch)` 성공 → 다음 `load_config(app_data_dir)`이 동일 값 반환 (roundtrip)
- **원자성**: `save_config` 실패 시 메모리 상태는 **변경되지 않는다** (write-through 순서: save 먼저, 성공해야 메모리 반영)
- `AppConfigPatch`에 `vault_path` 필드 없음 → 컴파일러가 실수 차단
- `watch_debounce_ms`는 v0.5 Settings UI 범위 밖 (Patch에 포함되지 않음)
- `ConfigState` RwLock poison 시 기존 패턴과 동일하게 `AppError::VaultNotFound(...)`로 매핑
- `detect_editors()`는 플랫폼/환경과 무관하게 panic하지 않음 (빈 Vec 허용)
- `resolve_editor`는 editor_command가 `://` 를 포함하는지로 URL/Command 분기

## Behavior Contract — detect_editors

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 임의 플랫폼 | `detect_editors` | `Vec<DetectedEditor>` 반환, panic 없음 |
| 2 | macOS, `/Applications/Visual Studio Code.app` 존재 | `detect_editors` | 결과에 `id == "vscode"` 포함 |
| 3 | macOS, `/Applications/Obsidian.app` 존재 | `detect_editors` | 결과에 `id == "obsidian"`, `command == "obsidian://open?path={path}"` 포함 |
| 4 | Unix, `which code` 성공 (앱 디렉토리 없어도) | `detect_editors` | 결과에 `id == "vscode"` 포함 |
| 5 | 어떤 에디터도 감지 안 됨 | `detect_editors` | 빈 Vec |

## Behavior Contract — resolve_editor

| # | Given | When | Then |
|---|-------|------|------|
| 6 | `"obsidian://open?path={path}"`, `/a/b.md` | `resolve_editor` | `Url("obsidian://open?path=%2Fa%2Fb.md")` |
| 7 | `"/usr/local/bin/code {path}"`, `/a/b.md` | `resolve_editor` | `Command { program: "/usr/local/bin/code", args: vec!["/a/b.md"] }` |
| 8 | `"/usr/local/bin/code"` (플레이스홀더 없음), `/a/b.md` | `resolve_editor` | `Command { program: "/usr/local/bin/code", args: vec!["/a/b.md"] }` (append 기본 동작) |
| 9 | URL이지만 `{path}` 없음, `/a/b.md` | `resolve_editor` | `Url("<원문 그대로>")` — URL에 {path} 없는 경우는 그대로 연다 (프론트가 이 경우 경고 UX) |
| 10 | 공백/빈 문자열, `/a/b.md` | `resolve_editor` | `Command { program: "", args: vec!["/a/b.md"] }` (open_in_editor에서 spawn 실패 → 에러 반환) |

## Behavior Contract — update_config

| # | Given | When | Then |
|---|-------|------|------|
| 11 | `patch.editor_command = Some("nvim")` 만 | `update_config` | 저장 후 `editor_command == "nvim"`, 다른 필드 유지 |
| 12 | 모든 필드 `None` (빈 patch) | `update_config` | 기존 config 그대로, `save_config`는 호출 (no-op 저장 허용) |
| 13 | update 성공 후 | `get_config` | patch 반영된 스냅샷 반환 |
| 14 | update 성공 후 | `load_config(app_data_dir)` | 영속된 값 동일 |
| 15 | `save_config` 실패 | `update_config` | 에러 반환 + ConfigState 원상 유지 (원자성) |

## Edge Cases

- `editor_command`에 `{path}` 플레이스홀더가 여러 개 포함 → 전부 치환 (`replace`)
- URL 인코딩은 대상 경로의 `/`, 공백, 특수문자만 최소 인코딩
- `exclude_dirs` patch 적용 시 인덱스/watcher 재구축은 **v0.5 비범위** (사용자가 rescan 트리거하거나 앱 재시작)
- `global_shortcut` patch 적용 시 런타임 재등록은 **v0.5 비범위** (앱 재시작 필요)
- `detect_editors` 호출은 파일 시스템 I/O를 수반 — UI에서 앱 시작 시 1회만 호출 + "재검색" 버튼

## Dependencies

- 기존 `config::{save_config, load_config, AppConfig, ConfigState}`
- 신규: `urlencoding` 크레이트 (또는 수동 minimal 인코딩)
- `tauri_plugin_opener::open_url` (Url 분기의 실제 호출은 `open_in_editor`에서)
- Mock boundary: `detect_editors`의 파일/쉘 탐지는 테스트 어려움 → 핵심 로직은 `resolve_editor` 유닛 테스트로 커버

## 기존 코드 영향

- `commands/note.rs::open_in_editor`
  - `resolve_editor`로 분기
  - `Command` → 기존 `std::process::Command::spawn`
  - `Url` → `opener::open_url`
- `lib.rs`
  - 새 커맨드 3개 handler 등록: `get_config`, `update_config`, `detect_editors`

## Mock boundary

- `resolve_editor`는 순수 함수 — 쉽게 유닛 테스트
- `update_config`/`get_config`는 ConfigState와 파일 I/O 결합 → TempDir 기반 통합 테스트
- `detect_editors`는 실제 시스템 의존 → smoke 테스트만 (빈 Vec 또는 일부 존재 확인)
