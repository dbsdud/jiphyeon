# Spec: transcribe (v1.0 Epic 3 녹음 + 볼트 스킬 연동)

## 전제

- **철학**: Jiphyeon은 **녹음 캡처/저장**만 담당. 전사/요약/md 작성은 볼트의 `vault-transcribe` 스킬이 수행.
- 사이드바에 이미 🎙️ Transcribe placeholder(disabled) 존재 — 이를 활성화
- `rescaffold` 인프라로 스킬/훅 템플릿 배포 가능 (`src-tauri/templates/vault/.claude/`)
- MediaRecorder API 포맷 지원이 플랫폼별로 다름:
  - macOS (WKWebView): `audio/mp4;codecs=mp4a.40.2` (AAC) → `.m4a`
  - Windows (WebView2): `audio/webm;codecs=opus` → `.webm`
  - Linux (WebKitGTK): 엔진별 차이 — `.webm` 또는 `.ogg`
- 60분 초과 시 **이어서 녹음**: 현재 MediaRecorder stop → 즉시 새 인스턴스 start, 파일명에 시퀀스 번호

## Slice 구조

- **Slice 3A**: 사이드바 활성화 + `/transcribe` 페이지 + 녹음 UI(파형, 타이머, 60분 rolling) + Rust `save_recording` 커맨드
- **Slice 3B**: Ctrl/Cmd+V 오디오 파일 붙여넣기 + 볼트 scaffold 확장 (`vault-transcribe` 스킬 + `check-pending-recordings.sh` 훅 + `settings.json`의 SessionStart 등록)

## Public Interface

### Rust 커맨드 (Slice 3A)

```rust
// src-tauri/src/commands/transcribe.rs (신규)
/// 녹음 바이너리를 활성 볼트의 _sources/recordings/ 하위에 저장.
/// - 디렉토리가 없으면 생성
/// - 반환: 저장된 절대 경로
#[tauri::command]
pub fn save_recording(
    state: State<'_, VaultState>,
    config_state: State<'_, ConfigState>,
    filename: String,
    bytes: Vec<u8>,
) -> Result<String, AppError>;
```

### TypeScript API

```ts
// src/lib/api.ts
export function saveRecording(filename: string, bytes: Uint8Array): Promise<string> {
  return invoke("save_recording", { filename, bytes: Array.from(bytes) });
}
```

### 프론트 유틸 (순수 함수)

```ts
// src/lib/transcribe/format.ts (신규)
export interface RecordingFormat {
  mimeType: string;
  extension: string;
}

/** MediaRecorder가 지원하는 첫 번째 포맷을 반환. 지원하는 게 없으면 null. */
export function pickSupportedFormat(): RecordingFormat | null;

/** YYYY-MM-DD-HHmm-<seq>.<ext> 형태 파일명 */
export function buildFilename(now: Date, sequence: number, ext: string): string;
```

폴백 순서:
1. `audio/mp4;codecs=mp4a.40.2` → `.m4a`
2. `audio/webm;codecs=opus` → `.webm`
3. `audio/ogg;codecs=opus` → `.ogg`

### 프론트 페이지

`/src/routes/transcribe/+page.svelte`:
- 상단: 큰 ◉ Record / ◼ Stop 버튼 + 경과 시간(HH:MM:SS)
- 중앙: Canvas 파형 (record 중에만 활성, idle 시 안내 문구)
- 하단: 이번 세션에서 저장된 파일 리스트 (파일명 + 경로)
- Ctrl/Cmd+V 오디오 붙여넣기 영역 (Slice 3B)

### 사이드바 변경 (Slice 3A)

`src/routes/+layout.svelte`의 navGroups "작업" 그룹:
```ts
{ href: "/transcribe", label: "Transcribe", icon: "🎙️" }
```
- `disabled`, `disabledReason` 필드 제거
- 🎙️ 아이콘 유지

### 스킬 scaffold 템플릿 (Slice 3B)

**자동화 수준: 반자동** — SessionStart 훅이 미전사 녹음을 감지해 Claude에 컨텍스트 주입. Claude가 사용자에게 제안하거나 사용자가 `/vault-transcribe` 호출로 실행.

#### `src-tauri/templates/vault/.claude/skills/vault-transcribe/SKILL.md`

- frontmatter: `name: vault-transcribe`, `description: _sources/recordings/의 녹음을 전사해 노트로 생성`
- 키워드 트리거: `/vault-transcribe`, "전사", "녹음 정리"
- 기본 모델: `base` (상수로 SKILL.md 내 표기. 사용자 변경 방법 명시)
- 절차:
  1. `_sources/recordings/`의 파일 중 `_sources/transcripts/<stem>.md`가 없는 것들 나열
  2. 사용자에게 대상 확인 (전체 / 특정 파일 / skip)
  3. 각 파일에 대해:
     - `openai-whisper` CLI 실행: `whisper <path> --model base --output_format txt --output_dir /tmp`
     - 전사 텍스트 읽기
     - 요약/주요 토픽/잠재 태그 생성
     - 관련 노트 검색(vault-new와 동일 로직)
     - `_sources/transcripts/<stem>.md` 작성 — frontmatter(type: transcript, source: 원본경로, duration, model) + 요약 + 본문(전사 원문) + 관련 노트 링크
  4. 완료 후 생성된 transcript 경로 나열

#### `src-tauri/templates/vault/.claude/hooks/check-pending-recordings.sh`

SessionStart 훅. 미전사 녹음 탐지 → stderr로 알림:
- `_sources/recordings/` 순회
- 각 파일의 stem이 `_sources/transcripts/<stem>.md`로 존재하지 않으면 pending
- 0개면 silent exit
- 1개 이상이면 `stderr`로 "📼 미전사 녹음 N개 있음. `/vault-transcribe`로 처리 가능." 출력
- `|| true`로 훅 실패가 세션 시작을 막지 않음

#### `src-tauri/templates/vault/.claude/settings.json`

기존 SessionStart 배열에 hook 항목 추가:
```json
{
  "hooks": [
    { "type": "command", "command": "bash .claude/hooks/check-pending-recordings.sh" }
  ]
}
```

## Invariants

- 저장 경로: `{vault_path}/_sources/recordings/` 고정 (MVP)
- 파일명: `YYYY-MM-DD-HHmm-<2자리 seq>.{ext}` (예: `2026-04-20-1453-01.m4a`)
- 60분(`3_600_000`ms) 도달 시 현재 파일 저장 완료 후 즉시 다음 파일 시작
- 시퀀스 번호는 **녹음 세션 내부 카운터** (앱 재실행/새 녹음 시 01로 리셋)
- 녹음 중 페이지 이탈 시 현재 MediaRecorder를 stop하고 수집된 데이터는 저장 (leak 방지)
- 활성 볼트가 없으면 `save_recording` 에러 반환 (UI에서 "볼트를 먼저 선택하세요" 안내)
- 마이크 권한 거부 시 UI에 안내 + record 버튼 비활성
- 저장 디렉토리(`_sources/recordings/`)가 없으면 자동 생성

## Behavior Contract — Rust (Slice 3A)

| # | Given | When | Then |
|---|-------|------|------|
| 1 | vault_path=None | `save_recording("a.m4a", bytes)` | `AppError::VaultNotConfigured` |
| 2 | vault_path 있음, `_sources/recordings/` 없음 | `save_recording("a.m4a", bytes)` | 디렉토리 생성 + 파일 저장 + 절대 경로 반환 |
| 3 | 같은 파일명이 이미 존재 | `save_recording(..)` | 덮어쓰기 없이 에러 반환 (`AppError::FileExists`) |
| 4 | filename에 상위 경로 포함 (`../etc/passwd`) | `save_recording` | `AppError::InvalidPath` (path traversal 방지) |
| 5 | filename이 허용 확장자(m4a/webm/ogg) 외 | `save_recording` | `AppError::InvalidExtension` |
| 6 | bytes 크기 0 | `save_recording` | `AppError::EmptyRecording` |

## Behavior Contract — 프론트 순수 함수 (Slice 3A)

| # | Given | When | Then |
|---|-------|------|------|
| 7 | 환경에서 mp4/webm/ogg 모두 지원 | `pickSupportedFormat` | `{ mimeType: "audio/mp4;codecs=mp4a.40.2", extension: ".m4a" }` (첫번째) |
| 8 | 환경에서 mp4 미지원, webm 지원 | `pickSupportedFormat` | `{ mimeType: "audio/webm;codecs=opus", extension: ".webm" }` |
| 9 | 환경에서 어떤 포맷도 미지원 | `pickSupportedFormat` | `null` |
| 10 | now=2026-04-20T14:53, seq=1, ext=".m4a" | `buildFilename` | `"2026-04-20-1453-01.m4a"` |
| 11 | seq=12 | `buildFilename` | 2자리 제로패딩 `"...12.{ext}"` |

## Behavior Contract — 프론트 UI (수동 검증)

| # | Given | When | Then |
|---|-------|------|------|
| 12 | 사이드바 Transcribe 클릭 | 클릭 | `/transcribe` 이동, disabled 아님 |
| 13 | 첫 진입 | 렌더 | Record 버튼 활성, 파형은 비활성 안내 |
| 14 | Record 클릭, 마이크 권한 허용 | 녹음 시작 | Canvas 파형 실시간 렌더, 타이머 증가, 버튼이 Stop으로 전환 |
| 15 | Record 중 Stop 클릭 | stop | MediaRecorder dataavailable → save_recording 호출 → 하단 리스트에 파일 추가 |
| 16 | Record 60분 경과 | 자동 | 현재 파일 저장 + 즉시 다음 파일로 이어감 (seq 증가), 파형/타이머 연속 |
| 17 | 마이크 권한 거부 | Record 클릭 | 에러 안내, 버튼 비활성 유지 |
| 18 | 녹음 중 페이지 이탈(라우팅/창 닫기) | navigate | 현재 MediaRecorder stop + 저장 시도 |
| 19 | 활성 볼트 없음 | Record 시도 | 안내 문구 "볼트를 먼저 선택하세요" |
| 20 | 라이트/다크 테마 각각 | 렌더 | 파형 색상(accent 토큰) 반영 |

## Behavior Contract — Slice 3B (수동 검증)

| # | Given | When | Then |
|---|-------|------|------|
| 21 | Finder에서 오디오 파일 복사(Cmd+C) 후 /transcribe 에서 Cmd+V | paste | 해당 파일을 `save_recording`으로 동일 경로에 저장, 시퀀스 증가, 원래 확장자 유지 |
| 22 | 비-오디오 파일(`application/pdf`) 붙여넣기 | paste | 무시 + 안내 토스트 "오디오 파일만 지원합니다" |
| 23 | 여러 오디오 파일 동시 붙여넣기 | paste | 각각 순차 저장 |
| 24 | rescaffold mode="add-missing" 실행 | rescaffold | `skills/vault-transcribe/SKILL.md`, `hooks/check-pending-recordings.sh`, `settings.json` 갱신 모두 포함 |
| 25 | 사용자 커스텀 `settings.json` 있는 볼트에 rescaffold | rescaffold | 기존 훅 보존 + SessionStart 배열에 pending-recordings 항목 머지 (중복 추가 방지) |
| 26 | `_sources/recordings/`에 미전사 파일 2개 있는 볼트에서 `claude` 세션 시작 | SessionStart | 훅이 "📼 미전사 녹음 2개 있음. `/vault-transcribe`로 처리 가능." stderr 출력 |
| 27 | 모든 녹음이 이미 transcript 보유 | SessionStart | 훅이 silent exit (출력 없음) |
| 28 | `_sources/recordings/` 디렉토리 부재 | SessionStart | 훅이 정상 exit 0 (에러 없음) |
| 29 | `/vault-transcribe` 호출 | Claude 실행 | 스킬 절차대로 whisper base 모델로 전사 + `_sources/transcripts/<stem>.md` 생성 (frontmatter + 요약 + 원문 + 링크) |

## Dependencies

### Rust
- `src-tauri/src/commands/transcribe.rs` — 신규 (save_recording 커맨드)
- `src-tauri/src/commands/mod.rs` — 모듈 등록
- `src-tauri/src/lib.rs` — invoke_handler 등록
- `src-tauri/src/error.rs` — `FileExists`, `InvalidPath`, `InvalidExtension`, `EmptyRecording` 배리언트 추가

### Scaffold 템플릿 (Slice 3B)
- `src-tauri/templates/vault/.claude/skills/vault-transcribe/SKILL.md` — 신규
- `src-tauri/templates/vault/.claude/hooks/check-pending-recordings.sh` — 신규
- `src-tauri/templates/vault/.claude/settings.json` — SessionStart 배열에 항목 추가
- `src-tauri/src/commands/rescaffold.rs` — settings.json 머지 로직 확인 (훅 항목 중복 방지)

### 프론트
- `src/routes/transcribe/+page.svelte` — 신규
- `src/lib/transcribe/format.ts` — 신규 (순수 함수)
- `src/lib/transcribe/recorder.ts` — 신규 (MediaRecorder 래퍼, rolling 로직)
- `src/lib/transcribe/waveform.ts` — 신규 (Canvas 파형 렌더)
- `src/lib/api.ts` — `saveRecording` 추가
- `src/routes/+layout.svelte` — Transcribe nav item 활성화
- `src/lib/types.ts` — (필요시) 타입 추가

## Mock Boundary

- Rust: 파일 시스템 I/O는 tempfile로 테스트 (기존 스캐폴드/노트 테스트 패턴)
- 프론트 `format.ts`: 순수 함수 — 수동 E2E (프로젝트 frontend 테스트 프레임워크 미도입)
- MediaRecorder / Web Audio API: 실제 브라우저 필요 → 수동 E2E

## 테스트 목록 (Rust, Slice 3A)

`src-tauri/src/commands/transcribe.rs #[cfg(test)]`:

1. `save_recording_creates_recordings_dir_if_missing`
2. `save_recording_writes_bytes_to_expected_path`
3. `save_recording_rejects_path_traversal`
4. `save_recording_rejects_unknown_extension`
5. `save_recording_rejects_empty_bytes`
6. `save_recording_errors_when_file_exists`
7. `save_recording_errors_when_vault_not_configured`

## Edge Cases

- **파일명 충돌**: 같은 분 시간대에 60분 rolling으로 여러 파일. 시퀀스 번호가 해결. 드문 경우 사용자가 수동으로 같은 이름 지정할 수 없음 (파일명 자동 생성)
- **대용량 파일 IPC**: 60분 m4a(AAC 64kbps) ≈ 28MB. `Vec<u8>`로 JSON serialize 되는 건 base64 → 37MB. 1회성이라 수용.
- **권한 API 차이**: `navigator.mediaDevices.getUserMedia({ audio: true })` — 실패 시 UI 폴백 필수
- **마이크 분리(hot-unplug)**: 녹음 중 장치 제거 → MediaRecorder `error` 이벤트. 현재까지 수집된 데이터만 저장
- **브라우저 탭 백그라운드**: Tauri 데스크톱 앱이라 일반적으로 문제 없음. 단 macOS의 앱 비활성 상태에서 마이크 접근 제한은 OS 정책
- **MediaRecorder ondataavailable 타이밍**: stop 직후 마지막 chunk 전달. 이때 Blob 완성 후 저장
- **볼트 전환 중 녹음**: 녹음 중에 볼트 전환 시도 → 경고 모달 (녹음 중단 후 전환 or 취소) — 초기 MVP는 단순히 저장 경로만 새 볼트로 바뀌게 두고, 문제 발생 시 추후 보완

## Out of Scope (Epic 3)

- **완전 자동 전사** (앱이 파일 watcher로 감지 + Claude CLI subprocess 호출) — v0.9 이후 고려
- 녹음 재생 UI (저장된 파일 미리듣기)
- 녹음 품질 설정 (비트레이트/샘플레이트)
- 녹음 중 북마크/챕터 기록
- 화면 녹화, 영상 입력
- YouTube/웹 URL 자막 추출 — 별도 볼트 스킬로 후속 검토
- 노이즈 제거/트리밍 등 오디오 편집
- whisper.cpp 번들(오프라인 전사) — openai-whisper CLI(Python)를 사용자가 별도 설치

## 열린 결정 (이미 확정)

- 녹음 포맷: 플랫폼 폴백(m4a → webm → ogg) ← **확정** (2026-04-20)
- 파형: Canvas + AnalyserNode ← **확정**
- 저장 경로: `_sources/recordings/` 고정 ← **확정**
- 전사 트리거: 앱은 저장만. 자동화는 볼트 레포에서 v0.9 전 정교화 ← **확정**
- 최대 시간: 60분 rolling (자동 이어감) ← **확정**
- 사이드바 아이콘: 🎙️ 유지, disabled 제거 ← **확정**
- Slice 구조: 2개(A=녹음UI+저장, B=클립보드+스킬/훅 scaffold) ← **확정**
- 자동화 수준: **반자동** (SessionStart 훅 알림 → 사용자 `/vault-transcribe` 호출) ← **확정**
- whisper 모델 기본값: **base** (SKILL.md 내 상수, 사용자 변경 가능) ← **확정**
- 전사 노트 구조: frontmatter + 요약 + 키워드 + 원문 + 관련 노트 링크(b 옵션) ← **확정**
- 훅 파일명: `check-pending-recordings.sh` (기존 "check-" 컨벤션) ← **확정**
