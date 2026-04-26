# Spec: Graph Report Markdown Parser (Slice C-2)

**상태**: Draft
**작성일**: 2026-04-26
**연관 로드맵**: `docs/plans/v2.0-pivot-roadmap.md` Slice C-2
**선행**: Slice C-1 (`spec-graphify-reader.md`)

## 목표

활성 프로젝트의 `graphify-out/GRAPH_REPORT.md` 를 파싱해 대시보드(Slice C-6) 에서 카드로 표시할 도메인 모델을 만든다. 이 슬라이스는 **파서 + 단위 테스트**만. IPC 는 C-3, 시각화는 C-6.

## 배경 (실측 bloghub 기준 섹션 구조)

```
# Graph Report - <project_root>  (YYYY-MM-DD)

## Corpus Check          # 코퍼스 규모 / verdict
## Summary               # 노드/엣지/커뮤니티 카운트, 추출율, 토큰비용
## Community Hubs (Navigation)
## God Nodes (most connected - your core abstractions)
## Surprising Connections (you probably didn't know these)
## Hyperedges (group relationships)
## Communities
### Community 0 - "label"
Cohesion: 0.02
Nodes (71): a(), b(), c() (+63 more)

### Community 1 - "label"
...
```

- `## Hyperedges` 는 graph.json 에서 더 풍부하게 받을 수 있어 보고서 측은 무시 (모델에서 제외).
- `## Community Hubs` 는 위키링크 목록만이라 메타로 가치 적음 — 무시.
- 대신 `## Communities` 의 `### Community N - "label"` + Cohesion + 샘플 노드만 추출.

## 데이터 모델

```rust
// src-tauri/src/graphify/report.rs
pub struct GraphReportSummary {
    pub nodes_count: Option<usize>,
    pub edges_count: Option<usize>,
    pub communities_count: Option<usize>,
    pub extracted_pct: Option<f64>,
    pub inferred_pct: Option<f64>,
    pub ambiguous_pct: Option<f64>,
    pub token_input: Option<u64>,
    pub token_output: Option<u64>,
}

pub struct GraphReportGodNode {
    pub rank: usize,
    pub name: String,
    pub edge_count: usize,
}

pub struct GraphReportSurprisingConnection {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub confidence: GraphifyConfidence,    // C-1 의 enum 재사용
}

pub struct GraphReportCommunity {
    pub id: i64,
    pub label: String,
    pub cohesion: Option<f64>,
    pub nodes_count: Option<usize>,
    pub sample_nodes: Vec<String>,
}

pub struct GraphReport {
    pub generated_at: Option<String>,    // 제목의 (YYYY-MM-DD)
    pub project_root: Option<String>,    // 제목의 path
    pub summary: GraphReportSummary,
    pub god_nodes: Vec<GraphReportGodNode>,
    pub surprising_connections: Vec<GraphReportSurprisingConnection>,
    pub communities: Vec<GraphReportCommunity>,
}
```

## Public Interface

```rust
/// graphify-out 디렉토리에서 GRAPH_REPORT.md 를 파싱.
/// 파일이 없으면 GraphifyError::NotRun (C-1 과 동일 에러 타입 재사용).
pub fn read_graphify_report(graphify_out_dir: &Path) -> Result<GraphReport, GraphifyError>;
```

## Behavior Contract

### `read_graphify_report`

- Given: `graphify-out` 미존재
- When: 호출
- Then: `Err(GraphifyError::NotRun)`
- Given: 디렉토리 존재, GRAPH_REPORT.md 미존재
- When: 호출
- Then: `Err(GraphifyError::NotRun)`
- Given: GRAPH_REPORT.md 가 빈 파일
- When: 호출
- Then: `Ok(report)` — 모든 섹션 빈 결과, summary 의 모든 필드 None
- Given: 제목 라인 `# Graph Report - /path  (2026-04-24)`
- When: 호출
- Then: `report.project_root == Some("/path")`, `report.generated_at == Some("2026-04-24")`
- Given: Summary 라인 `1166 nodes · 1934 edges · 148 communities detected`
- When: 호출
- Then: 각각 1166/1934/148 채워짐
- Given: Summary 라인 `Extraction: 63% EXTRACTED · 37% INFERRED · 0% AMBIGUOUS · ...`
- When: 호출
- Then: extracted_pct=63.0, inferred_pct=37.0, ambiguous_pct=0.0
- Given: Summary 라인 `Token cost: 0 input · 0 output`
- When: 호출
- Then: token_input=0, token_output=0
- Given: God Nodes 라인 ``1. `GET()` - 76 edges``
- When: 호출
- Then: god_nodes 첫 항목 = {rank:1, name:"GET()", edge_count:76}
- Given: Surprising 라인 `` - `runCycle()` --calls--> `releaseStuckLocks()`  [INFERRED] ``
- When: 호출
- Then: 첫 항목 = {source:"runCycle()", target:"releaseStuckLocks()", relation:"calls", confidence:Inferred}
- Given: Communities 섹션의 `### Community 0 - "Community 0"` 후 `Cohesion: 0.02` 후 `Nodes (71): a(), b() (+63 more)`
- When: 호출
- Then: communities 첫 항목 = {id:0, label:"Community 0", cohesion:Some(0.02), nodes_count:Some(71), sample_nodes:["a()", "b()"]}
- Given: God Nodes 섹션 자체가 없음
- When: 호출
- Then: `god_nodes` 가 빈 vec, 다른 섹션 정상 파싱

### Edge Cases

- Surprising connection 의 confidence 가 unknown 문자열 → `GraphifyConfidence::Unknown`
- Cohesion 값이 정수 (`Cohesion: 1`) → `Some(1.0)`
- Communities 섹션의 `Nodes (N): ...` 가 없으면 sample_nodes 빈 vec
- Summary 한 줄에 일부 필드만 있으면 (e.g. `1166 nodes`) 가능한 만큼 채우고 나머지 None
- BOM 처리: graph.json 과 동일하게 strip (C-1 패턴 재사용 가능)

## Dependencies

- `regex` (이미 vault/parser 에서 사용 중) — 추가 의존성 없음
- C-1 의 `GraphifyConfidence` 재사용 → `graphify::reader::GraphifyConfidence` 를 그대로 import

## 비범위

- IPC (`get_graphify_report`) → C-3
- 대시보드 카드 UI → C-6
- 보고서 내 위키링크/링크 변환 → 없음 (raw 텍스트만)

## 작업 순서 (TDD)

1. `tests/fixtures/graphify/` 에 픽스처 추가:
   - `report-minimal.md`: 빈 파일 또는 헤더만
   - `report-sample.md`: 모든 섹션 한두 항목씩 (실측 bloghub 의 대표 패턴 축약)
2. `graphify::report` 모듈 신규 + 모델 정의
3. BC 12 케이스 테스트 (Red)
4. 라인 스캐너 + regex 기반 구현 (Green)
5. clippy / cargo test 통과
6. 커밋: `feat: graphify GRAPH_REPORT.md 파서 (Slice C-2)`

## 완료 조건

- BC 12 케이스 green
- `read_graphify_report` 와 `read_graphify_graph` 가 같은 디렉토리를 받아 독립적으로 동작
- C-1 과 동일한 `GraphifyError::NotRun` 시맨틱
- Rust + clippy 전부 green
