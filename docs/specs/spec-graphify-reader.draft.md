# Spec: Graphify Graph Reader (Slice C-1)

**상태**: Draft
**작성일**: 2026-04-26
**연관 로드맵**: `docs/plans/v2.0-pivot-roadmap.md` Slice C-1
**선행**: Epic B 완료 (활성 프로젝트의 `graphify_out_path` 확정)

## 목표

활성 프로젝트의 `graphify-out/graph.json` 을 파싱해 Rust 측 모델로 변환한다.
이 슬라이스는 **파서 + 단위 테스트**만 다룬다. IPC 커맨드 / 프론트 / watcher 는 후속 슬라이스 (C-2 ~ C-7).

## 배경

- graphify (외부 Python 도구) 는 NetworkX `node_link_data` 형식 변종을 출력한다. 실측 파일 (`bloghub/graphify-out/graph.json`) 기준 스키마:
  - 최상위: `directed`, `multigraph`, `graph`, `nodes`, `links`
  - **`links` 키** (v2.x NetworkX 기본 — `edges` 가 아님). 각 link 는 `source` / `target` 외에 `_src` / `_tgt` 같은 보조 필드도 가짐 (현재 사용 X)
  - `graph.hyperedges`: 3+ 노드를 묶는 의미 단위. 별도 컬렉션으로 보존
- 노드 / 엣지의 community / confidence / source_file / source_location / norm_label 등 graphify 고유 메타가 풍부. v2.0 그래프 시각화는 이 메타에 의존.
- v1.0 시절 `models.rs::GraphNode` / `GraphEdge` 는 자체 인덱서 산출물이라 폐기. C-1 은 별도 모델을 도입.

## 데이터 모델 (Rust)

```rust
// src-tauri/src/graphify/mod.rs (신규)
pub mod reader;

pub use reader::{
    read_graphify_graph, GraphifyConfidence, GraphifyEdge, GraphifyError, GraphifyGraph,
    GraphifyHyperedge, GraphifyNode,
};
```

```rust
// src-tauri/src/graphify/reader.rs

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum GraphifyConfidence {
    Extracted,
    Inferred,
    Ambiguous,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphifyNode {
    pub id: String,
    pub label: String,
    pub file_type: Option<String>,        // code | document | paper | image | None
    pub source_file: Option<String>,
    pub source_location: Option<String>,
    pub community: Option<i64>,
    pub norm_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphifyEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub confidence: GraphifyConfidence,
    pub confidence_score: f64,
    pub source_file: Option<String>,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphifyHyperedge {
    pub id: String,
    pub label: String,
    pub nodes: Vec<String>,
    pub relation: String,
    pub confidence: GraphifyConfidence,
    pub confidence_score: f64,
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphifyGraph {
    pub nodes: Vec<GraphifyNode>,
    pub edges: Vec<GraphifyEdge>,
    pub hyperedges: Vec<GraphifyHyperedge>,
}

#[derive(Debug, thiserror::Error)]
pub enum GraphifyError {
    #[error("graphify-out 디렉토리에 graph.json 이 없습니다. /graphify 를 먼저 실행하세요.")]
    NotRun,
    #[error("graph.json 읽기 실패: {0}")]
    Io(#[from] std::io::Error),
    #[error("graph.json 파싱 실패: {0}")]
    Parse(String),
}
```

## Public Interface

```rust
/// graphify-out 디렉토리(`<project>/graphify-out/`) 를 받아 graph.json 을 파싱한다.
/// 디렉토리 자체가 없거나 파일이 없으면 `GraphifyError::NotRun`.
pub fn read_graphify_graph(graphify_out_dir: &Path) -> Result<GraphifyGraph, GraphifyError>;
```

내부 구조:
- 직렬화 입력 모델 `RawGraphFile`, `RawNode`, `RawLink`, `RawHyperedge` 를 별도 정의해 serde 로 받기
- `RawGraphFile.links` 를 `GraphifyEdge` 로 변환 (source/target 필수, weight 기본 1.0, confidence_score 기본 1.0)
- `RawGraphFile.graph.hyperedges` (있을 때만) 를 `GraphifyHyperedge` 로 변환

## Behavior Contract

### `read_graphify_graph`

- Given: `<dir>` 자체가 없음
- When: 호출
- Then: `Err(GraphifyError::NotRun)`
- Given: `<dir>` 은 있으나 `graph.json` 없음
- When: 호출
- Then: `Err(GraphifyError::NotRun)`
- Given: `graph.json` 이 잘못된 JSON
- When: 호출
- Then: `Err(GraphifyError::Parse(_))`
- Given: 노드 1 + 엣지 0 의 최소 그래프
- When: 호출
- Then: `Ok(graph)` — `graph.nodes.len() == 1`, `edges.is_empty()`, `hyperedges.is_empty()`
- Given: `nodes`, `links` (NetworkX 키), `graph.hyperedges` 가 모두 있는 일반 그래프
- When: 호출
- Then: 각 컬렉션 길이가 입력과 일치
- Given: link 의 `confidence` 가 `"EXTRACTED"` / `"INFERRED"` / `"AMBIGUOUS"`
- When: 파싱
- Then: 대응 enum variant 로 직렬화/역직렬화. 미지의 값은 `Unknown` (panic 없음)
- Given: link 의 `weight` / `confidence_score` 누락
- When: 파싱
- Then: 각각 `1.0` 으로 폴백
- Given: node 의 `file_type` / `source_file` / `community` / `norm_label` 누락
- When: 파싱
- Then: 각각 `None` (오류 없음)
- Given: hyperedge 의 `nodes` 가 빈 배열
- When: 파싱
- Then: hyperedge 자체는 보존 (필터링은 시각화 레이어 책임)

### Edge Cases

- 매우 큰 graph.json (>10 MB): 한 번에 전체 deserialize. 캐싱 / streaming 은 없음. 1500 노드 ~ 2000 엣지 (실측 bloghub) 가정. 추가 최적화는 측정 후 결정.
- BOM 포함된 JSON: serde_json 이 거부할 수 있음 — 입력 첫 3 바이트가 `EF BB BF` 면 strip 후 파싱.
- node.id 중복: serde 는 그대로 둠. 시각화 측이 알아서 처리.

## Dependencies

- 기존 `serde`, `serde_json` 활용
- `thiserror` 기반 신규 에러 타입 추가
- 새 외부 의존성 없음

## 비범위

- IPC 커맨드 (`get_graphify_graph` 등) → C-3
- watcher → C-4
- GRAPH_REPORT.md 파서 → C-2
- 프론트엔드 시각화 → C-5

## 작업 순서 (TDD)

1. `tests/fixtures/graphify/` 에 두 픽스처 작성:
   - `minimal.json`: 노드 1, 링크 0, hyperedges 없음
   - `sample.json`: 노드 3, 링크 2 (EXTRACTED + INFERRED), hyperedge 1 (3 노드)
2. `graphify::reader::tests` 에 BC 8개 테스트 작성 (Red)
3. `RawGraphFile` 구조 + `read_graphify_graph` 구현 (Green)
4. confidence enum / 누락 필드 폴백 / BOM 처리 / hyperedge nodes 빈 배열 검증
5. `cargo test --lib` 통과
6. `mod graphify;` 를 `lib.rs` 에 추가
7. clippy 통과
8. 커밋: `feat: graphify graph.json 파서 (Slice C-1)`

## 완료 조건

- 8개 BC 테스트 green
- `cargo test --lib` + clippy 전부 통과
- 새 모듈은 lib.rs 에 등록되지만 invoke_handler 는 아직 미연결 (C-3에서)
