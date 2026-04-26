# Spec: Cross-Project Graph (Slice C-7)

**상태**: Draft
**작성일**: 2026-04-26
**선행**: Slice C-1 ~ C-6
**연관 로드맵**: `docs/plans/v2.0-pivot-roadmap.md` Slice C-7
**브랜치**: `feat/v2.0-epic-c7-cross-project`

## 목표

여러 프로젝트의 `graph.json` 을 합쳐 단일 그래프로 시각화한다. 동일 개념(`norm_label` 기준)을 가진 노드들을 가상 브리지 엣지로 연결해 "두 프로젝트에서 같은 개념" 을 한눈에 보여준다. 집현의 핵심 차별점.

## 데이터 모델

```rust
// src-tauri/src/graphify/cross.rs (신규)
pub struct CrossProjectMember {
    pub project_id: String,
    pub project_name: String,
}

// 기존 GraphifyNode 를 확장하지 않고, 멀티 프로젝트 메타를 담는 별도 구조 사용.
pub struct CrossProjectNode {
    pub id: String,                  // "{project_id}::{node_id}" 네임스페이스
    pub label: String,
    pub original_id: String,         // 원본 graph.json 의 id
    pub project_id: String,
    pub community: Option<i64>,
    pub file_type: Option<String>,
    pub source_file: Option<String>,
    pub norm_label: Option<String>,
}

pub struct CrossProjectEdge {
    pub source: String,              // 네임스페이스된 id
    pub target: String,
    pub relation: String,
    pub confidence: GraphifyConfidence,
    pub confidence_score: f64,
    pub project_id: Option<String>,  // 브리지 엣지면 None
    pub is_bridge: bool,             // norm_label 기반 동일 개념 연결
}

pub struct CrossProjectGraph {
    pub nodes: Vec<CrossProjectNode>,
    pub edges: Vec<CrossProjectEdge>,
    pub members: Vec<CrossProjectMember>,
}
```

**결정**:
- 기존 `GraphifyHyperedge` 는 머지 X (프로젝트 내부 그룹은 cross 시각화에서 의미 약함)
- 브리지 엣지는 `is_bridge=true` 로만 표시 → 프론트가 시각화 차별

## Public Interface

```rust
// src-tauri/src/commands/graphify.rs (확장)
#[tauri::command]
pub fn get_cross_project_graph(
    state: State<'_, ConfigState>,
    project_ids: Vec<String>,
    merge_labels: bool,
) -> Result<CrossProjectGraph, GraphifyError>;
```

```ts
// frontend
export interface CrossProjectMember { project_id: string; project_name: string; }
export interface CrossProjectNode { ... }
export interface CrossProjectEdge { ... is_bridge: boolean; project_id: string | null }
export interface CrossProjectGraph { nodes; edges; members; }

export function getCrossProjectGraph(projectIds: string[], mergeLabels: boolean): Promise<CrossProjectGraph>;
```

## Behavior Contract

### `merge_graphs` (pure, 단위 테스트 가능)

```rust
fn merge_graphs(
    members: &[CrossProjectMember],
    graphs: Vec<(String /* project_id */, GraphifyGraph)>,
    merge_labels: bool,
) -> CrossProjectGraph;
```

- Given: 빈 입력
- When: merge
- Then: 빈 graph
- Given: 프로젝트 1 개의 그래프 (n=2, e=1)
- When: merge
- Then: 노드 2 (id 가 `{p1}::orig` 네임스페이스), 엣지 1 (project_id=p1, is_bridge=false)
- Given: 프로젝트 2 개, 동일 norm_label "jwt validator" 가 양쪽에 있음, merge_labels=true
- When: merge
- Then: 두 노드를 잇는 가상 브리지 엣지 1 추가 (is_bridge=true, relation="cross_project_alias", confidence=Inferred, score=0.5, project_id=None)
- Given: 동일 norm_label, merge_labels=false
- When: merge
- Then: 브리지 엣지 없음 (단순 union)
- Given: norm_label 이 None 인 노드들
- When: merge
- Then: 브리지 후보에서 제외
- Given: 동일 norm_label 가 같은 프로젝트 안에 N 개
- When: merge
- Then: 동일 프로젝트 내 노드끼리는 브리지 X (이미 같은 그래프)

### `get_cross_project_graph` IPC

- Given: project_ids 비어있음
- When: 호출
- Then: 모든 등록 프로젝트로 처리
- Given: project_ids 일부가 graphify 미실행
- When: 호출
- Then: 해당 프로젝트는 무시 (NotRun 으로 실패한 프로젝트는 스킵, 로그)
- Given: 모든 프로젝트가 graphify 미실행
- When: 호출
- Then: `Err(GraphifyError::NotRun)`
- Given: 정상
- When: 호출
- Then: `Ok(graph)` + members 에 실제 합쳐진 프로젝트 목록

## Frontend

### `/graph` 페이지 확장

- 상단 헤더에 "전체 프로젝트" 토글 (single ↔ cross)
- cross 모드:
  - 프로젝트 멀티 셀렉트 체크박스 (기본: 등록된 모든 프로젝트 체크)
  - "라벨 병합" 토글 (merge_labels)
  - LinkGraph 가 cross 데이터 받음 (브리지 엣지는 굵은 점선 + 별도 색상)
- single 모드: 기존 그대로

### `LinkGraph.svelte` 확장

- 입력 union: `GraphifyGraph | CrossProjectGraph`
  - 단순화: 별도 prop `bridges?: CrossProjectEdge[]` 또는 `mode: "single" | "cross"` + 데이터 어댑터
  - 더 단순: 컴포넌트는 단일 데이터 모델만 받고 어댑터가 GraphifyGraph 를 CrossProjectGraph 로 변환 (single 모드도 wrapper 한 번 거침)
- 결정: **어댑터 패턴**. LinkGraph 는 항상 `{ nodes: NodeView[], edges: EdgeView[] }` 형태만 받음. node 에 `project_id` 옵션, edge 에 `is_bridge` / `project_id` 옵션. single 모드도 이 형태로 변환해 전달.

### 시각 차별

- 노드 색상 (cross 모드): 기존 community 색상은 그대로, 노드 외곽 ring 색상은 project_id 해시 팔레트
  - 단순 구현: 기존 fill = community 색, stroke = project 색 (single 모드도 동일 — single 은 stroke = fill 동일)
- 브리지 엣지: stroke `#ff79c6` (눈에 띄는 색) + `dash [6,3]` + width `1.5` + opacity `0.7`
- 일반 cross 프로젝트 엣지: 현재 동일

### 노드 클릭

- single 모드: 기존 동작
- cross 모드:
  - source_file 이 있으면 (md 든 아니든) 그 노드의 project 로 자동 switch + single 모드로 포커스 (id 검색 포함)
  - 단순화: 일단 토스트 안내만 ("Switched to project X") + 페이지 reload. 디테일한 포커스는 후속.

## 비범위

- 노드 검색이 cross 모드에서 동일 동작 (label.includes) — community 필터는 single 만
- "두 프로젝트에서 같은 개념" 패널 (브리지 클릭) — 후속
- 대규모 (>3000) 시 자동 필터링 권유 — 카운트 표시만

## 작업 순서

1. `graphify::cross` 모듈 신규 + `merge_graphs` pure fn + 6 BC 단위 테스트
2. `commands/graphify.rs` 에 `get_cross_project_graph` 추가 (활성 안 된 프로젝트는 graphify_out_path 가 hub link 라 OK)
3. lib.rs invoke_handler 등록
4. types.ts / api.ts 신규 타입 + 함수
5. LinkGraph.svelte 입력을 어댑터 형태로 일반화 (single 어댑터, cross 어댑터)
6. /graph 페이지에 토글 + cross 패널 (체크박스 + merge_labels)
7. 검증: cargo test + clippy + svelte-check
8. 커밋 분리:
   - backend: `feat: cross-project graph merge + IPC (Slice C-7 backend)`
   - frontend: `feat: /graph cross 모드 토글 + 브리지 시각화 (Slice C-7 frontend)`

## 완료 조건

- merge_graphs 6 BC green
- 등록된 모든 프로젝트 묶어서 cross 그래프 표시 가능
- merge_labels=true 시 동일 norm_label 노드 간 브리지 엣지
- 노드 색상: community fill + project ring (cross 모드)
- 브리지 엣지 시각 차별
- Rust + clippy + svelte-check 전부 green
