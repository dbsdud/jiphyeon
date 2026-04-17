# Spec: quick-note (Phase 4)

## Public Interface

### Backend

```rust
/// 퀵 노트를 inbox 폴더에 생성
#[tauri::command]
pub fn create_quick_note(
    config: State<'_, AppConfig>,
    title: Option<String>,
    content: String,
    tags: Vec<String>,
) -> Result<String, AppError>  // 생성된 파일의 상대 경로 반환
```

### AppConfig 확장

```rust
pub struct AppConfig {
    // 기존 필드 유지
    pub vault_path: PathBuf,
    pub watch_debounce_ms: u64,
    pub recent_notes_limit: usize,
    pub exclude_dirs: Vec<String>,
    pub editor_command: String,
    // 추가
    pub quick_note_folder: String,    // 기본 "inbox"
    pub global_shortcut: String,      // 기본 "CmdOrCtrl+Shift+N"
}
```

### 글로벌 단축키 등록 (lib.rs setup)

```rust
// tauri_plugin_global_shortcut 사용
// 단축키 감지 → WebviewWindowBuilder로 캡처 윈도우 생성/포커스
// 윈도우: label="capture", url="/capture", 480x360, always_on_top, resizable=false
```

### Frontend

```typescript
// api.ts
export function createQuickNote(
  title: string | null,
  content: string,
  tags: string[]
): Promise<string>
```

```svelte
<!-- routes/capture/+page.svelte — 캡처 윈도우 전용 -->
제목 input (optional)
내용 textarea (autofocus)
태그 input (comma-separated)
저장 / 취소 버튼
```

## Behavior Contract

| # | Given | When | Then |
|---|-------|------|------|
| 1 | title="오늘의 메모", content="내용", tags=["dev"] | create_quick_note | `inbox/2026-04-16-오늘의-메모.md` 생성, 상대 경로 반환 |
| 2 | title=None, content="빠른 메모" | create_quick_note | `inbox/2026-04-16-143052.md` (타임스탬프 기반 파일명) |
| 3 | inbox 폴더 미존재 | create_quick_note | 자동 생성 후 파일 저장 |
| 4 | 앱 백그라운드 상태 | Cmd+Shift+N | 캡처 윈도우 표시 (always_on_top) |
| 5 | 캡처 윈도우 이미 열림 | Cmd+Shift+N | 기존 윈도우 포커스 |
| 6 | 캡처 윈도우에서 Escape | 키 입력 | 윈도우 닫기 (저장 안 함) |
| 7 | 저장 성공 | 저장 버튼 클릭 | 윈도우 닫기, watcher가 변경 감지 → 인덱스 갱신 |
| 8 | 빈 content | 저장 시도 | 프론트엔드에서 validation 에러 (전송 안 함) |

## Edge Cases

- 동일 파일명 충돌 → 파일명에 `-1`, `-2` 등 suffix 추가
- content에 frontmatter 구분자(`---`) 포함 → 무해 (본문 영역이므로)
- 단축키 등록 실패 (다른 앱이 사용 중) → 에러 로그만 남기고 앱 시작 계속
- macOS 접근성 권한 미부여 → 단축키 미동작, 앱 내 버튼으로 대체 가능

## Frontmatter 템플릿

```yaml
---
type: idea
created: {YYYY-MM-DD}
status: seedling
tags: {tags or []}
---
```

## 파일 저장

- 경로: `{vault_path}/{quick_note_folder}/{YYYY-MM-DD}-{slug_or_timestamp}.md`
- inbox/ 없으면 자동 생성

## Dependencies

- `tauri-plugin-global-shortcut = "2"` — 글로벌 단축키
- `@tauri-apps/plugin-global-shortcut: ^2` — 프론트엔드 (필요 시)
- `slug` (기존 의존성) — 파일명 slug 생성
- `chrono` (기존 의존성) — 날짜/시간
- Mock boundary: 파일 시스템 I/O (`fs::write`)

## 레이아웃 분기

- `+layout.svelte`에서 `page.url.pathname`이 `/capture`일 때 사이드바 숨김
- 캡처 윈도우는 독립적인 미니 UI로 동작
