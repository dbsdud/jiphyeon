# Spec: Capture URL Tab (Slice D-2)

**상태**: Draft
**작성일**: 2026-04-26
**선행**: D-1

## 목표

`/capture` 의 🔗 URL 탭에 클리핑 동작을 추가. 기존 `WebClipDialog.svelte` 로직을 흡수해 capture 단일 창에서 처리. 저장 경로는 활성/선택 프로젝트의 `docs/clippings/`.

## 변경

### Backend

- `commands/clipper.rs::clip_url` 시그니처에 `project_id: Option<String>` 추가 (D-1 의 quick_note 패턴과 동일)
- `clipper/mod.rs::clip_url_with_html` 의 저장 폴더를 `docs/inbox` → `docs/clippings` 로 변경 (로드맵 D-3 결정 반영)
- 기존 단위 테스트 갱신 (inbox → clippings)

### Frontend

- `api.ts::clipUrl(request, projectId?)`
- `/capture` URL 탭: URL 입력 + 태그 입력 + 클립 버튼. 동작은 D-1 의 노트 탭과 동일한 흐름 (저장 후 closeWindow)
- `WebClipDialog.svelte` 는 D-3 에서 제거 (현재는 layout 의 사이드바 ✂️ Clip 액션에 여전히 연결되어 있어 유지)

## Behavior Contract

### `clip_url` (확장)

- Given: project_id=None
- When: 호출
- Then: 활성 프로젝트의 `docs/clippings/YYYY-MM-DD-{slug}.md` 에 저장
- Given: project_id=Some(valid id)
- When: 호출
- Then: 해당 프로젝트의 docs/clippings/, 활성 변경 X
- Given: project_id=Some(unknown)
- When: 호출
- Then: `Err(AppError::VaultNotConfigured)`

## 비범위

- WebClipDialog 모달 제거 / 사이드바 Clip 액션 변경 → D-3
- frontmatter 에 source_url 추가 (기존에 이미 source: <url> 들어있음 — 충분)

## 작업 순서

1. backend: `clipper/mod.rs::clip_url_with_html` 의 inbox → clippings 디렉토리, 단위 테스트 2개 수정
2. backend: `commands/clipper.rs::clip_url` 에 project_id 인자 추가
3. frontend: `api.ts::clipUrl(request, projectId?)`
4. frontend: `/capture` URL 탭 동작 구현
5. cargo test + clippy + svelte-check
6. 커밋: `feat: capture URL 탭 + docs/clippings 저장 (Slice D-2)`

## 완료 조건

- /capture URL 탭에서 URL 입력 → Clip → 선택 프로젝트의 `docs/clippings/` 에 저장
- WebClipDialog 모달은 여전히 사이드바에서 동작 (D-3 에서 제거 예정)
- 모든 검증 green
