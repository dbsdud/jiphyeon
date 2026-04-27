# Spec: Search IPC + 자동 재인덱싱 (Slice E-2)

**상태**: Draft
**작성일**: 2026-04-27
**선행**: E-1

## 목표

E-1 의 `SearchIndex` 를 IPC 로 노출하고, 부팅 시 / graphify 갱신 시 자동 재인덱싱.

## 변경

### Backend

- 신규 state: `SearchState = Arc<RwLock<SearchIndex>>`
- `lib.rs::setup`:
  - `open_or_create(app_data_dir/search-index/)`
  - `app.manage(search_state)`
  - 백그라운드 thread 로 `reindex_all` 호출 (시작 지연 방지)
- 신규 IPC (`commands/search.rs` 신규):
  - `search(query, project_filter?, kind_filter?, limit?) -> Vec<SearchHit>`
  - `reindex_active_project() -> usize`
  - `reindex_all_projects() -> usize`
- `mod search` 의 `#[allow(dead_code)]` 제거
- frontend 가 graphify-updated 이벤트 시 `reindex_active_project` 호출

### Frontend

- `api.ts`: `searchAll`, `reindexActiveProject`, `reindexAllProjects`
- `types.ts`: `SearchKind`, `SearchHit`
- `+layout.svelte`: graphify-updated listener 에서 `reindexActiveProject()` 호출 (best-effort, 실패 시 console.warn)
- 검색 UI 페이지는 E-3

## Behavior Contract (단위 테스트 가능 부분)

- E-1 의 search/reindex 가 이미 검증됨. E-2 는 IPC wrapper 라 단위 테스트 추가 없이 실측으로 충분.
- 백그라운드 reindex 의 동작은 통합 테스트 범위 (skip).

## 비범위

- /search 페이지 → E-3
- Cmd+K 팔레트 → E-4
- 한국어 토크나이저
- 인덱싱 진행 상태 토스트

## 작업 순서

1. backend: `commands/search.rs` 신규 + IPC 3개
2. lib.rs: SearchState 추가, setup 에서 open_or_create + 백그라운드 reindex, invoke_handler
3. mod search 의 #[allow(dead_code)] 제거
4. frontend api.ts/types.ts
5. +layout.svelte: graphify-updated → reindexActiveProject
6. cargo test + clippy + svelte-check
7. 단일 커밋: `feat: search IPC + 자동 재인덱싱 (Slice E-2)`

## 완료 조건

- 부팅 시 백그라운드에서 모든 프로젝트 인덱싱
- graphify-updated 이벤트 시 활성 프로젝트 자동 재인덱싱
- search IPC 가 노출되고 frontend 에서 호출 가능
- Rust + clippy + svelte-check 전부 green
