# Spec: Capture Shell (Slice D-1)

**상태**: Draft
**작성일**: 2026-04-26
**브랜치**: `feat/v2.0-epic-d-capture`
**연관 로드맵**: Epic D / Slice D-1+D-2

## 목표

`/capture` 단일 창에서 **퀵노트 / URL 클리핑 / 녹음** 세 가지 입력을 탭으로 통합하기 위한 골격을 깐다.
이번 슬라이스는 **탭 UI + 활성 프로젝트 셀렉터 + 퀵노트 탭 동작**까지. URL/녹음 탭은 후속 슬라이스에서 채움.

## 기존 자산

- `/routes/capture/+page.svelte` — 현재 단순 quick note 입력만
- `commands/note.rs::create_quick_note(title, content, tags)` — 활성 프로젝트의 `docs/inbox/` 에 저장
- 글로벌 단축키 Cmd+Shift+N 으로 capture 윈도우 띄움 (480×360)

## 변경 사항

### Backend

- `create_quick_note` 시그니처에 `project_id: Option<String>` 추가
  - `None` → 활성 프로젝트 (현재 동작)
  - `Some(id)` → 해당 프로젝트의 `docs_path/inbox/` 에 저장 (활성 전환 X)
- 같은 패턴을 D-2/D-3 에서 `clip_url`, `save_recording` 에 적용 예정 — 이번 슬라이스에서는 quick_note 만

### Frontend

- `/capture/+page.svelte`:
  - 윗줄: **활성 프로젝트 셀렉터** (드롭다운, 기본 = active project)
  - 다음 줄: **탭 그룹** [📝 노트 · 🔗 URL · 🎙️ 녹음]
    - 노트 탭: 기존 quick note UI (제목/본문/태그)
    - URL 탭: 자리만 (D-2)
    - 녹음 탭: 자리만 (D-3)
  - 프로젝트가 등록 0개면 안내 메시지
  - capture 윈도우 크기를 확대 (480×360 → 560×460) — 탭 + 셀렉터 추가에 따라
- 저장 후 토스트는 capture 윈도우가 즉시 닫히므로 의미 없음 → 메인 윈도우의 vault-changed/graphify-updated 가 자동 반영

## Behavior Contract

### `create_quick_note` (확장)

- Given: project_id=None, 활성 프로젝트 있음
- When: 호출
- Then: 활성 프로젝트의 `docs/inbox/` 에 저장 (현재 동작 유지)
- Given: project_id=Some(unknown id)
- When: 호출
- Then: `Err(AppError::VaultNotConfigured)` 와 동일하게 처리 (또는 새 분기 — 단순화)
- Given: project_id=Some(valid id), 활성 프로젝트와 다른 id
- When: 호출
- Then: 해당 프로젝트의 docs/inbox/ 에 저장. **active_project_id 는 변경 X** (capture 는 사이드 이펙트 없이 작동)

### Frontend `/capture` 상태 머신

```
loading → ready → tab=note (default) → save → close window
                  └→ tab=url    → "준비 중" (D-2)
                  └→ tab=record → "준비 중" (D-3)
```

- 윈도우 마운트 시 `listProjects()` + `getActiveProject()` 병렬 호출
- 셀렉터의 기본값 = active project id
- 활성 프로젝트 없으면 "프로젝트를 먼저 등록하세요" 안내 + 저장 버튼 비활성

## 비범위

- URL 클리핑 탭 동작 → D-2
- 녹음 탭 동작 → D-3
- WebClipDialog 모달 / `/transcribe` 페이지 제거 → D-3 또는 D-4
- pending graphify 뱃지 → D-4

## 작업 순서

1. backend: `commands/note.rs::create_quick_note` 에 `project_id: Option<String>` 추가, 단위 테스트는 spec contract 만 (실제 IPC 시그니처 변경)
2. lib.rs invoke_handler 시그니처 자동 (camelCase 변환은 Tauri 자동)
3. frontend: `api.ts::createQuickNote(title, content, tags, projectId?)`
4. frontend: `/capture/+page.svelte` 재작성 (탭 + 셀렉터 + 노트 탭 동작)
5. lib.rs 의 capture 윈도우 빌더에서 `inner_size(560.0, 460.0)`
6. cargo test + clippy + svelte-check
7. 단일 커밋: `feat: capture 창 탭 골격 + 프로젝트 셀렉터 (Slice D-1)`

## 완료 조건

- Cmd+Shift+N 으로 capture 창 열림
- 활성 프로젝트 셀렉터 + 3 탭 표시
- 노트 탭에서 다른 프로젝트 선택 후 저장 시 그 프로젝트의 docs/inbox/ 에 파일 생성, 메인 활성은 변경 X
- URL/녹음 탭은 placeholder
