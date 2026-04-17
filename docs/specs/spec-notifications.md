# Spec: notifications (v0.6 훅 기반 알림 인프라)

## 전제

- 앱은 **뷰어** — 훅을 실행하지 않는다. 훅 스크립트가 append한 jsonl을 **읽어서 토스트로 발화**만 한다.
- **규약 파일**: `<vault>/.claude/state/notifications.jsonl` (line-delimited JSON, append-only)
  - 각 라인 = 하나의 알림 이벤트. 훅 미수정 환경에서도 graceful (파일 없음 OK, 깨진 라인은 해당 라인만 스킵).
- **증분 읽기**: watcher가 파일 변경 감지 시 **새 라인만** 발화. 초기 기동 시 **기존 라인은 무시** (백로그 플러시 방지).
- 볼트 전환/재시작 시 증분 오프셋은 초기화 (해당 시점 라인 수 기준으로 재시작).
- 앱은 notifications.jsonl을 **쓰지 않는다** (외부 훅 스크립트 전용).

## 파일 포맷

```
{"level":"info","message":"...", "source":"pre-tool-use", "ts":"2026-04-17T12:34:56Z"}
{"level":"error","message":"..."}
```

- `level`: `"info" | "warn" | "error" | "success"` — 필수. 그 외 값은 해당 라인 스킵.
- `message`: 비어있지 않은 문자열 — 필수.
- `source`: 훅/출처 식별자 — 선택. UI 부가 표시용.
- `ts`: ISO-8601 타임스탬프 문자열 — 선택. 없으면 생략.
- 빈 라인은 스킵. 파일은 UTF-8로 간주.

## Public Interface

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Info,
    Warn,
    Error,
    Success,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationEvent {
    pub level: NotificationLevel,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<String>,
}

/// 볼트 루트 기준 notifications.jsonl 경로
pub fn notifications_path(vault_root: &Path) -> PathBuf

// --- 순수 파서 (유닛 테스트 대상) ---

/// jsonl 전체 내용을 파싱. 깨진/빈 라인은 스킵하고 유효 이벤트만 반환.
pub fn parse_notifications_jsonl(content: &str) -> Vec<NotificationEvent>

/// 라인 수 기반 증분 산출.
/// `prev_line_count` 이후에 추가된 라인만 파싱해 반환한다.
/// 파일이 줄어들었다면(= rotate/truncate) 전체를 다시 처리하고 새 line_count를 돌려준다.
///
/// 반환: (new_events, new_line_count)
pub fn read_new_notifications(
    content: &str,
    prev_line_count: usize,
) -> (Vec<NotificationEvent>, usize)

// --- Tauri 측 발화 지점 ---

/// watcher가 `notifications.jsonl` 변경을 감지할 때 호출.
/// - 내부적으로 파일을 읽고 `read_new_notifications`로 증분만 추출
/// - 각 이벤트를 `AppHandle.emit("notification", event)`로 발화
/// - 오프셋(라인 수)은 Mutex 상태로 보관
pub(crate) fn drain_and_emit(
    app_handle: &AppHandle,
    vault_root: &Path,
    state: &NotificationsState,
)

/// 증분 오프셋 상태 (볼트당 1개). 볼트 전환 시 `reset()`으로 초기화.
pub type NotificationsState = Arc<Mutex<NotificationsOffset>>;

pub struct NotificationsOffset {
    pub line_count: usize,
}

impl NotificationsOffset {
    pub fn new_from_current(vault_root: &Path) -> Self;  // 현재 라인 수로 초기화
    pub fn reset(&mut self, vault_root: &Path);
}
```

## Invariants

- `parse_notifications_jsonl`은 panic하지 않는다 (깨진 JSON, 빈 라인, 알 수 없는 level → 해당 라인 스킵).
- `notifications.jsonl` 파일 미존재 → 빈 Vec + `line_count == 0`.
- `drain_and_emit`는 **새 라인만** emit한다 (초기 기동 시점 이전 내용은 발화하지 않음).
- 파일이 **줄어들면** (truncate/rotate) → `prev_line_count > 현재 line_count`. 이 경우 전체를 처음부터 재처리하고 `line_count`를 현재 값으로 갱신 (중복 발화는 로그 성격상 허용).
- `level` 필수 4값이 아니면 해당 라인만 스킵 (엄격 매칭, 확장 없음).
- 앱은 이 파일을 **쓰지 않는다** (읽기 전용).
- 볼트 전환 시 `NotificationsState`는 새 볼트 기준으로 `reset`된다 (이전 볼트 오프셋 누수 방지).

## Behavior Contract — parse_notifications_jsonl

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 빈 문자열 | `parse_notifications_jsonl` | `[]` |
| 2 | `{"level":"info","message":"hi"}` 1라인 | parse | 1개 이벤트, `Info` / "hi" |
| 3 | 유효 3라인 | parse | 3개 이벤트, **입력 순서 보존** |
| 4 | 유효 2 + 깨진 JSON 1 | parse | 2개 이벤트 반환, 깨진 라인 스킵 |
| 5 | 빈 라인 / 공백 라인 포함 | parse | 스킵 |
| 6 | `{"level":"danger","message":"x"}` (알 수 없는 level) | parse | 해당 라인 스킵 |
| 7 | `{"level":"info"}` (message 누락) | parse | 해당 라인 스킵 |
| 8 | `{"level":"info","message":""}` (빈 message) | parse | 해당 라인 스킵 |
| 9 | `{"level":"info","message":"x"}` (source/ts 없음) | parse | OK, `source: None`, `ts: None` |
| 10 | `{"level":"success","message":"saved","source":"scaffold","ts":"2026-04-17T..."}` | parse | OK, 모든 필드 채워짐 |
| 11 | 끝에 trailing newline | parse | 영향 없음 (빈 라인 스킵 규칙으로 흡수) |

## Behavior Contract — read_new_notifications

| # | Given | When | Then |
|---|-------|------|------|
| 12 | content 5라인, `prev == 5` | read_new | `([], 5)` |
| 13 | content 5라인, `prev == 3` | read_new | 마지막 2라인만 파싱, 반환 line_count == 5 |
| 14 | content 3라인, `prev == 5` (축소) | read_new | 전체 3라인 재파싱, 반환 line_count == 3 |
| 15 | content 0라인, `prev == 0` | read_new | `([], 0)` |
| 16 | content 5라인 중 1라인 깨짐, `prev == 0` | read_new | 유효 4개 이벤트, line_count == 5 (스킵된 라인도 카운트) |
| 17 | `prev == 3`, 새 2라인 추가 중 1라인 깨짐 | read_new | 유효 1개만 반환, line_count == 5 |

> 라인 카운트는 **이벤트 유효성과 무관**하게 원본 라인 수. 깨진 라인을 다음에 또 파싱하지 않기 위함.

## Behavior Contract — notifications_path

| # | Given | When | Then |
|---|-------|------|------|
| 18 | `/vault` | `notifications_path` | `/vault/.claude/state/notifications.jsonl` |

## Behavior Contract — drain_and_emit (통합)

| # | Given | When | Then |
|---|-------|------|------|
| 19 | notifications.jsonl 미존재 | drain_and_emit | emit 0회, 에러 없음 |
| 20 | 초기 기동 직후 (state = current line count) + 변경 없음 | drain_and_emit | emit 0회 |
| 21 | 초기화 이후 2라인 append (둘 다 유효) | drain_and_emit | `"notification"` 이벤트 2회 emit, state.line_count += 2 |
| 22 | append된 라인 중 1개 깨짐 | drain_and_emit | 유효 건만 emit, state.line_count는 원본 라인 수 증분만큼 증가 |
| 23 | 파일 truncate 후 append 1라인 | drain_and_emit | 전체 재처리 후 1회 emit, state.line_count == 1 |

## Edge Cases

- **notifications.jsonl이 .claude/ 하위** — 기본 `exclude_dirs`에 `.claude`가 있어 기존 watcher 필터에서 걸러진다.
  → `should_watch` 규칙에 **예외 경로 화이트리스트** 추가 필요:
  볼트 기준 `.claude/state/notifications.jsonl`는 확장자 필터/exclude_dirs를 **우회**하여 감시한다.
- 파일 확장자 `.jsonl`은 현재 `WATCH_EXTENSIONS`(md/html)에 없음 → 화이트리스트 경로 예외로 처리.
- notifications.jsonl 파일 쓰기 레이스 (훅이 쓰는 동안 앱이 읽기) → partial write 가능성. 파서는 라인 단위이므로 마지막 불완전 라인은 다음 스캔에서 완성되면 읽힘. `line_count`는 **완전히 개행으로 끝난 라인 수**만 카운트하여 다음 증분에서 동일 라인을 재처리.
- 거대 파일 (수 MB) → MVP는 전체 읽기. 향후 파일 seek 기반 증분 읽기로 개선 여지.
- 동일 ts/message의 중복 append → 중복 그대로 emit (의미 레이어는 훅 쪽 책임).
- 볼트 미연결 상태 → watcher 자체가 없으므로 호출되지 않는다.

## Frontend 통합

- Tauri 이벤트: `"notification"` (payload = `NotificationEvent`)
- `src/lib/api.ts`: `listen("notification", handler)` 구독 훅 추가
- `src/lib/components/Toast.svelte`: `type` 확장
  - 기존: `"success" | "error"`
  - 확장: `"info" | "warn"` 추가 (4단계). Tailwind 색상 매핑:
    - success → 기존 (bg-success/90)
    - error → 기존 (bg-danger/90)
    - warn → `bg-warn/90` (신규, Tailwind 테마에 추가)
    - info → `bg-surface-2/95` (중립)
- `+layout.svelte`: 앱 시작 시 `listen("notification", …)` 등록, payload를 Toast에 전달
- **큐잉**: MVP는 **가장 최근 알림으로 덮어쓰기** (단일 Toast). 짧은 시간 내 여러 알림 도달 시 마지막만 가시. 향후 개선 시 큐 도입.

## Dependencies

- 신규: `src-tauri/src/notifications.rs` — 타입/파서/오프셋/drain
- 수정:
  - `watcher/mod.rs` — `.claude/state/notifications.jsonl` 경로 화이트리스트 + 변경 감지 시 `drain_and_emit` 호출
  - `lib.rs` — `NotificationsState` 등록 + 볼트 전환 훅에서 `reset` 호출
  - `commands/onboarding.rs::activate_vault` — 새 볼트에 대해 `NotificationsState::reset` 호출
  - `src/lib/components/Toast.svelte` — level 확장
  - `src/routes/+layout.svelte` — `"notification"` 리스너
- 기존: `tauri::AppHandle::emit` (mock boundary: 통합 테스트에서는 임시로 트레이트 주입 없이 TempDir + 실제 emit 검증 생략, 로직은 순수 함수로 분리해 커버)

## Mock boundary

- `parse_notifications_jsonl` / `read_new_notifications` — 순수 함수, 문자열 입력만. **유닛 테스트로 완전 커버**.
- `drain_and_emit` — 파일 I/O + AppHandle emit 결합. TempDir 기반 테스트에서는 emit을 호출하지 않는 **추출된 중간 함수** (`collect_new_events(vault_root, state) -> Vec<NotificationEvent>`)를 통해 검증. 실제 emit 경로는 smoke.
- Frontend Toast 렌더링은 Svelte 컴포넌트 기본 동작에 위임 — 별도 테스트 없음.

## 향후 (이 Spec의 범위 외)

- `_maintenance/notifications-history.md` 등 영구 보관 로그 (rotate/archive)
- 필터링 UI (level 별), 모두 지우기 버튼
- macOS 네이티브 알림 (notify-rust) 병행 발화
- 백로그 플러시 옵션 ("앱 시작 시 최근 N개 알림 보여주기")
