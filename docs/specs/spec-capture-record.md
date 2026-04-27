# Spec: Capture Record Tab + Cleanup (Slice D-3)

**상태**: Draft
**작성일**: 2026-04-26
**선행**: D-1, D-2

## 목표

`/capture` 의 🎙️ 녹음 탭에 RollingRecorder 동작을 이식.
`/transcribe` 라우트와 `WebClipDialog` 모달을 삭제하고 사이드바도 정리.

## 변경

### Backend

- `commands/transcribe.rs::save_recording / list_recordings / delete_recording` 모두 `project_id: Option<String>` 추가 (D-1/D-2 패턴 동일)
- 신규 IPC `open_capture_window`: 글로벌 단축키 핸들러와 동일하게 capture 윈도우 띄우기 (이미 있으면 show+focus, 없으면 build)

### Frontend

- `api.ts`:
  - `saveRecording(filename, bytes, projectId?)`, `listRecordings(projectId?)`, `deleteRecording(filename, projectId?)`
  - `openCaptureWindow()`
- `/capture` 녹음 탭:
  - Record/Stop 버튼 + 경과 시간 + 파형 (`RollingRecorder` + `WaveformRenderer` 재활용)
  - Stop 시 chunk 저장 콜백에서 `closeWindow()`
  - 녹음 목록 표시는 생략 (capture 윈도우는 좁음 — 목록은 메인 explore 에서 보임)
- `/transcribe/+page.svelte` 디렉토리 삭제
- `WebClipDialog.svelte` 컴포넌트 삭제
- `+layout.svelte`:
  - 사이드바 nav 의 `action: "clip"` 항목 → `action: "capture"` 로 변경, 라벨 `✏️ Capture`, 클릭 시 `openCaptureWindow()` 호출
  - `/transcribe` 메뉴 제거
  - `WebClipDialog` import/사용 제거
- `vault.svelte.ts` 의 vaultRefresh 는 그대로 — 녹음 저장 시 watcher 가 vault-changed emit (확장자가 .m4a 등이라 .md 필터에 안 걸림 — 그래도 메인 explore 가 자동 새로고침 안 되어도 OK, 사용자 명시적 리프레시 필요 시)

## Behavior Contract

### `save_recording` (확장)

- Given: project_id=None
- When: 호출
- Then: 활성 프로젝트의 `docs/_sources/recordings/` 에 저장 (현재 동작 유지)
- Given: project_id=Some(id)
- When: 호출
- Then: 해당 프로젝트의 동일 경로

같은 패턴으로 `list_recordings`, `delete_recording` 동작.

### `open_capture_window`

- Given: capture 윈도우가 이미 떠 있음
- When: 호출
- Then: show + setFocus
- Given: 없음
- When: 호출
- Then: `WebviewWindowBuilder` 로 새로 build (560×460, "Capture", always_on_top)

## 비범위

- pending graphify 뱃지 → D-4
- 녹음 중 capture 윈도우 닫힘 시 자동 stop (사용자 실수 방지) — 후속에서 검토
- 녹음 chunk rolling 전략 변경 (현재 그대로)

## 작업 순서

1. backend: save_recording/list_recordings/delete_recording 시그니처 확장 + IPC `open_capture_window` 신규
2. lib.rs: invoke_handler 등록, 글로벌 단축키 핸들러 코드를 함수로 추출해 IPC 와 공유
3. frontend api.ts: 4 함수 갱신
4. /capture 녹음 탭 구현 (RollingRecorder + Waveform)
5. /transcribe 디렉토리 삭제
6. WebClipDialog.svelte 삭제
7. +layout.svelte: 사이드바 / 모달 정리
8. cargo test + clippy + svelte-check
9. 단일 커밋: `feat: capture 녹음 탭 + /transcribe·WebClipDialog 정리 (Slice D-3)`

## 완료 조건

- /capture 의 녹음 탭에서 record → stop → 자동 저장 + 윈도우 닫힘
- /transcribe / WebClipDialog 잔재 제거
- 사이드바 ✏️ Capture 클릭 시 capture 윈도우 열림 (글로벌 단축키와 동일)
- 모든 검증 green
