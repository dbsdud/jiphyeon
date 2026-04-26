# Spec: Graphify Watcher (Slice C-4)

**상태**: Draft
**작성일**: 2026-04-26
**선행**: Slice C-3, v2.5 Workspace Hub

## 목표

활성 프로젝트의 `graphify-out/graph.json` / `GRAPH_REPORT.md` 가 변경되면 프론트에 `graphify-updated` 이벤트를 emit. `/graph` 와 Dashboard v2 가 자동 갱신.

## 배경

- 현재 watcher 는 workspace_path 전체를 감시하면서 `.md` 와 `.html` 만 통과시킴 → graph.json 변경은 무시됨
- v1 시절 정의된 `vault-changed` 이벤트는 노트 변경 용으로 그대로 유지
- graphify 산출물은 사용자가 외부에서 `/graphify` 실행 시에 한 번 갱신됨 — 빈도 낮음, 변경 단위가 큰 파일 묶음

## 변경

### `watcher/mod.rs`

- `WATCH_EXTENSIONS` 에 `"json"` 추가? — 너무 광범위. 특정 파일명만 화이트리스트가 안전.
- 새 화이트리스트 패턴: 워크스페이스 내 `*/graphify-out/graph.json` 또는 `*/graphify-out/GRAPH_REPORT.md`. 경로 매칭으로 처리.
- 변경 감지 시:
  - `vault-changed` 가 아니라 새 이벤트 `graphify-updated` 를 emit
  - 페이로드: `{ project_root, kind: "graph" | "report" }`
  - debounce 는 기존 500ms 그대로 — 사용자가 graphify 한 번 실행하면 graph.json + REPORT.md 둘 다 짧은 시간 내 갱신되므로 양 쪽 다 받아도 OK

### `should_watch` 확장

- 기존 분기: notifications.jsonl 화이트리스트 → `.md`/`.html` + exclude_dirs
- 새 분기 (notifications.jsonl 다음에 검사): 경로의 마지막 두 segment 가 `graphify-out/graph.json` 또는 `graphify-out/GRAPH_REPORT.md` 이면 통과
- 그 외 `.json` 은 여전히 무시

### Frontend

- `src/routes/+layout.svelte` 에 `graphify-updated` 리스너 추가
- 새 store 또는 기존 vaultRefresh 재사용?
  - 옵션 A: 별도 `graphifyRefresh` store 신규 — `/graph` 와 Dashboard 만 구독
  - 옵션 B: 기존 `vaultRefresh.bump()` 재사용 — `/graph` 는 이미 vaultRefresh 추적 중이므로 추가 작업 없음
- **선택 B**: 단순함. Dashboard 와 /graph 가 이미 vaultRefresh 추적. 파일 변경 사이드 이펙트가 페이지 재로드 정도라 store 분리 가치 적음
  - 단, layout 의 vault-changed 리스너 외에 graphify-updated 도 같이 bump

## Behavior Contract

### `should_watch` (확장)

- Given: 경로가 `<workspace>/<proj>/graphify-out/graph.json`
- When: 호출
- Then: `true`
- Given: 경로가 `<workspace>/<proj>/graphify-out/GRAPH_REPORT.md`
- When: 호출
- Then: `true` (이미 .md 라 통과)
- Given: 경로가 `<workspace>/<proj>/graphify-out/cache/foo.json`
- When: 호출
- Then: `false` (graphify-out 직속만 허용)
- Given: 경로가 `<workspace>/<proj>/something.json`
- When: 호출
- Then: `false` (.json 일반은 무시)

### Watcher emit (debounced)

- Given: graph.json 변경 1회
- When: debounce 만료
- Then: `graphify-updated { project_root, kind: "graph" }` 1회 emit
- Given: graph.json + GRAPH_REPORT.md 동시 변경
- When: debounce
- Then: 각각 emit (또는 같은 디바운스 윈도우 안에서 둘 모두 emit). 프론트는 어느 쪽이든 vaultRefresh.bump() 로 동일 처리

## 구현 메모

- `should_watch` 의 graphify-out 분기를 추가하면 기존 `.md` 필터에 GRAPH_REPORT.md 가 이미 통과되지만 `vault-changed` 이벤트로 emit 됨 (vaultRefresh.bump 만 트리거하면 충분)
- graph.json 만 새 분기 + `graphify-updated` 이벤트로 분리하면 미래에 페이로드/UI 분리 여지가 있음
- 우선 단순하게: `.md` 그대로 + graph.json 만 `graphify-updated` emit. Frontend 는 두 이벤트 모두 vaultRefresh.bump.

## 작업 순서 (TDD)

1. `watcher/mod.rs::should_watch` 에 graphify-out 분기 + 4 BC 단위 테스트
2. emit 분기: `path.ends_with("graph.json")` 이면 `graphify-updated`, 그 외 `.md` 는 `vault-changed`
3. `+layout.svelte`: `listen("graphify-updated", ...)` 추가, 콜백에서 `vaultRefresh.bump()`
4. 검증: cargo test + clippy + svelte-check
5. 커밋: `feat: graphify-out watcher 분기 (Slice C-4)`

## 비범위

- graphify 실행 자체 (집현이 graphify 실행 X — v2.5 결정 그대로)
- 별도 store 분리
- payload 의 project_root 활용 (Frontend 는 단순 bump 만)
