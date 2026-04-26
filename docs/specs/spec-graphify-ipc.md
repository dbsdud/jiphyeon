# Spec: Graphify IPC Commands (Slice C-3)

**상태**: Draft
**작성일**: 2026-04-26
**연관 로드맵**: `docs/plans/v2.0-pivot-roadmap.md` Slice C-3
**선행**: Slice C-1, C-2

## 목표

활성 프로젝트의 graphify-out 을 프론트엔드에서 가져갈 IPC 3종을 추가한다. 기존 `read_graphify_graph` / `read_graphify_report` 를 활용해 활성 프로젝트의 `graphify_out_path` 를 자동 결정한다.

## Public Interface

```rust
// src-tauri/src/commands/graphify.rs (신규)

#[tauri::command]
pub fn get_graphify_graph(
    state: State<'_, ConfigState>,
) -> Result<GraphifyGraph, GraphifyError>;

#[tauri::command]
pub fn get_graphify_report(
    state: State<'_, ConfigState>,
) -> Result<GraphReport, GraphifyError>;

#[tauri::command]
pub fn get_graphify_status(
    state: State<'_, ConfigState>,
) -> Result<GraphifyStatus, AppError>;

#[derive(Debug, Clone, Serialize)]
pub struct GraphifyStatus {
    pub project_id: Option<String>,
    pub graphify_out_path: Option<String>,
    pub graph_json_exists: bool,
    pub report_md_exists: bool,
    pub last_run_at: Option<String>,    // graph.json mtime (RFC3339)
    pub nodes_count: Option<usize>,     // 빠른 파싱 — 없으면 None
    pub edges_count: Option<usize>,
}
```

### Frontend

```ts
// src/lib/types.ts
export interface GraphifyStatus {
  project_id: string | null;
  graphify_out_path: string | null;
  graph_json_exists: boolean;
  report_md_exists: boolean;
  last_run_at: string | null;
  nodes_count: number | null;
  edges_count: number | null;
}

export interface GraphifyConfidence ... // "EXTRACTED" | "INFERRED" | "AMBIGUOUS" | "UNKNOWN"
export interface GraphifyNode { ... }
export interface GraphifyEdge { ... }
export interface GraphifyHyperedge { ... }
export interface GraphifyGraph { nodes; edges; hyperedges }
export interface GraphReport { ... }

// src/lib/api.ts
export function getGraphifyGraph(): Promise<GraphifyGraph>;
export function getGraphifyReport(): Promise<GraphReport>;
export function getGraphifyStatus(): Promise<GraphifyStatus>;
```

## Behavior Contract

### `get_graphify_graph` / `get_graphify_report`

- Given: 활성 프로젝트 없음
- When: 호출
- Then: `Err(GraphifyError::NotRun)` (또는 새 variant `NoActiveProject` — 결정 필요)
- Given: 활성 프로젝트 있고 graph.json/REPORT.md 미존재
- When: 호출
- Then: `Err(GraphifyError::NotRun)` (이미 reader 가 처리)
- Given: 정상 graph.json/REPORT.md
- When: 호출
- Then: `Ok(graph/report)` — 내부적으로 `read_graphify_*` 호출

**결정 사항**: `NoActiveProject` 별도 variant 도입 안 함. 활성 프로젝트가 없을 때도 `NotRun` 으로 통일 — 프론트는 같은 빈 상태로 처리하면 충분. (대신 `get_graphify_status` 로 세분화)

### `get_graphify_status`

- Given: 활성 프로젝트 없음
- When: 호출
- Then: `Ok(GraphifyStatus { project_id: None, ..모두 false/None })`
- Given: 활성 프로젝트 있음, graphify-out 디렉토리 없음
- When: 호출
- Then: `project_id=Some(_)`, `graphify_out_path=Some(_)`, 나머지 false/None
- Given: graph.json 만 있음
- When: 호출
- Then: `graph_json_exists=true`, `report_md_exists=false`, `last_run_at=Some(_)`, nodes_count/edges_count=Some(_)
- Given: 둘 다 있음
- When: 호출
- Then: 모두 채워짐

**구현 노트**: `get_graphify_status` 는 노드/엣지 카운트만 알면 되므로 graph.json 을 한 번 읽되 deserialize 비용을 감수. 1500 노드 규모는 ms 수준. 별도 가벼운 메타 파일이 graphify 에 없으므로 이게 가장 단순.

## Dependencies

- C-1 의 `read_graphify_graph` + `GraphifyError`
- C-2 의 `read_graphify_report` + `GraphReport`
- 기존 `ConfigState` / `AppConfig::active_project()`

## 비범위

- watcher 로 `graphify-updated` 이벤트 emit → C-4
- 시각화/UI → C-5/C-6
- 크로스 프로젝트 그래프 → C-7

## 작업 순서 (TDD)

1. `commands/graphify.rs` 신규 + `GraphifyStatus` 타입
2. lib.rs: `mod commands; ... commands::graphify::*` invoke_handler 추가
3. graphify 모듈의 `#[allow(dead_code)]` 제거 (모듈 사용 시작)
4. 각 커맨드는 internal pure function + `#[tauri::command]` wrapper 분리, pure function 단위 테스트:
   - `compute_status(project, graphify_out_dir)` → 4 케이스
5. types.ts / api.ts 업데이트 (단순 wiring, 별도 테스트 X)
6. cargo test + clippy + svelte-check
7. 커밋 분리: backend / frontend 또는 단일 — 백엔드 위주라 단일 커밋이 자연스러움

## 완료 조건

- 3 IPC 등록되어 invoke_handler 에서 호출 가능
- `compute_status` 4 BC green
- `mod graphify` 의 `#[allow(dead_code)]` 제거 후 clippy 통과
- svelte-check 0/0
- 단일 커밋 `feat: graphify IPC 3종 (Slice C-3)`
