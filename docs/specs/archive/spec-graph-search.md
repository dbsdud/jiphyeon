# Spec: graph-search (v1.0 Epic 2.5 그래프 페이지 검색/필터)

## 전제

- Epic 2의 마지막 Slice. 대시보드 쪽 Slice(2.1 God Node, 2.2 Clusters)는 완료됨
- Slice 2.5는 원래 A~E(색상/크기/검색/포커스/품질오버레이)를 포함했으나, **스코프를 C(검색)+D(포커스)로 축소 후, 다시 C 검색으로 단일화**
- 검색 자체가 "앵커 선택 + 1-hop 자동 확장"의 의미를 가져 **별도의 포커스 모드는 불필요**
- 기존 그래프 페이지는 zoom/pan/drag 이미 지원. 노드는 `<a href>` 링크로 클릭 시 뷰어 이동 — 이 동작은 유지 (검색은 별도의 상단 바에서만 동작)

## 핵심 개념

**앵커(anchor)**: 검색어 및 필터를 모두 만족하는 노드 집합. 빈 검색 + 빈 필터일 땐 모든 노드가 앵커.

**활성(active)**: 앵커 ∪ 앵커의 1-hop 이웃. 그래프에서 정상 표시될 노드 집합.

**dim**: 활성 집합 밖의 노드/엣지. opacity 낮춰 흐리게 표시 (DOM 제거 아님 — force 시뮬레이션 튐 방지).

## Public Interface

### Rust — GraphNode 확장

```rust
// src-tauri/src/models.rs
#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub path: String,
    pub title: String,
    pub note_type: Option<NoteType>,
    pub link_count: usize,
    pub tags: Vec<String>,   // ← 추가
}
```

`get_link_graph` 커맨드가 `NoteEntry.frontmatter.tags`에서 tags를 채움. frontmatter 없으면 빈 `Vec`.

### TypeScript

```ts
// src/lib/types.ts
export interface GraphNode {
  id: string;
  path: string;
  title: string;
  note_type?: string;
  link_count: number;
  tags: string[];   // ← 추가
}
```

### 순수 필터 함수 (신규)

```ts
// src/lib/graph-filter.ts
export interface GraphFilter {
  query: string;            // trim 전 원본 — 내부에서 trim + lowercase
  typeFilter: string | null;   // null=전체, 특정 타입 문자열(e.g., "til")
  tagFilter: string | null;    // null=전체, 특정 태그
}

/**
 * 앵커 집합 + 1-hop 이웃을 포함하는 활성 노드 ID 집합을 반환.
 * 빈 쿼리 + 빈 필터 → 모든 노드 ID (전체 활성)
 * 매칭 앵커 없음 → 빈 집합 (전체 dim)
 */
export function computeActiveIds(
  nodes: GraphNode[],
  edges: GraphEdge[],
  filter: GraphFilter,
): Set<string>;

/**
 * 현재 필터 상태가 기본값(모두 비어있음)인지 확인.
 * 기본 상태면 컴포넌트가 dim 로직을 건너뛰고 전부 opacity 1로 렌더 가능.
 */
export function isFilterEmpty(filter: GraphFilter): boolean;
```

### Svelte — LinkGraph prop

```ts
// src/lib/components/LinkGraph.svelte
interface Props {
  // ... 기존
  activeNodeIds?: Set<string> | null;   // null = dim 없음(기본)
}
```

- `activeNodeIds == null`: 전체 노드 opacity 1 (기본 동작, 기존과 동일)
- 노드 opacity: `activeNodeIds.has(node.id) ? 1 : 0.15`
- 엣지 opacity: `activeNodeIds.has(src) && activeNodeIds.has(tgt) ? 1 : 0.1`
- CSS `transition: opacity 200ms ease`

### 그래프 페이지 상단 바

`/src/routes/graph/+page.svelte` 기존 헤더 영역에 한 줄 고정 바:

```
[🔍 search title...  ] [type ▾] [tag ▾] [Clear]
```

- 검색 input: placeholder "Search title..."
- type select: "All types" + 현재 볼트의 모든 type
- tag select: "All tags" + 현재 볼트의 모든 tag
- Clear 버튼: 검색/필터 초기화 (활성화 조건: 필터가 비어있지 않을 때)
- 검색/필터 상태가 변하면 즉시 dim 재계산 (debounce 불요 — 클라이언트 인메모리 필터링, 수백~수천 노드에서 밀리초)

## Invariants

- `activeIds ⊇ anchorIds`
- 빈 쿼리 ∧ 빈 type ∧ 빈 tag → `isFilterEmpty()` true → 컴포넌트는 `activeNodeIds={null}`로 호출
- 매칭 앵커 0개면 `activeIds = ∅` → 전체 dim
- 1-hop은 **무방향** (엣지 양끝 모두 이웃으로 인정)
- 검색어는 **title substring, case-insensitive**
- 필터는 **AND** (검색 + type + tag 모두 만족)
- 노드 클릭 시 기존 `<a href>` 동작(뷰어 이동) 유지 — 검색은 별도 UI에서만 조작
- `GraphNode.tags`는 frontmatter 없을 시 빈 배열 (옵셔널 아님, 항상 배열)

## Behavior Contract — Rust

| # | Given | When | Then |
|---|-------|------|------|
| 1 | frontmatter에 tags: [rust, arch] 있는 노트 | `get_link_graph` | GraphNode.tags == ["rust", "arch"] |
| 2 | frontmatter 없는 노트 (plain md) | `get_link_graph` | GraphNode.tags == [] |

## Behavior Contract — graph-filter.ts (순수 함수)

| # | Given | When | Then |
|---|-------|------|------|
| 3 | nodes=[A,B,C], edges=[], query="", type=null, tag=null | `computeActiveIds` | {A, B, C} |
| 4 | 같은 조건 | `isFilterEmpty` | true |
| 5 | query="rust", A.title="rust guide", B.title="kotlin", C.title="rust fn", edges=[A-B] | `computeActiveIds` | 앵커={A, C}, 1-hop={B} → active={A, B, C} |
| 6 | query="rust", 매칭 없음 | `computeActiveIds` | {} |
| 7 | type="til", A/B/C 중 A만 til | `computeActiveIds` | 앵커={A} + 이웃 |
| 8 | query="rust" AND type="til" | `computeActiveIds` | AND 조합 앵커 + 이웃 |
| 9 | tag="rust", A.tags=["rust"] | `computeActiveIds` | 앵커={A} + 이웃 |
| 10 | 노드 중복 없이 Set 반환 | - | 결과는 Set (dedup 자동) |
| 11 | query 양쪽 공백 " Rust " | `computeActiveIds` | 트림 + lowercase 후 매칭 |
| 12 | query="" but typeFilter="til" | `isFilterEmpty` | false |
| 13 | edges=[A-B, B-C], query="A"로 A 매칭 | `computeActiveIds` | 1-hop만 확장 → {A, B} (C는 2-hop이므로 제외) |

## Behavior Contract — 프론트 (수동 검증)

| # | Given | When | Then |
|---|-------|------|------|
| 14 | 그래프 페이지 첫 진입 | 렌더 | 상단 바 노출, 전체 노드 정상(dim 없음) |
| 15 | 검색창에 "rust" 입력 | 즉시 | 매칭 노트+이웃만 정상, 나머지 200ms transition으로 dim |
| 16 | type 드롭다운에서 "til" 선택 | 즉시 | til 노트+이웃만 정상 |
| 17 | 검색+type+tag 모두 설정 | 즉시 | AND 조합 적용 |
| 18 | Clear 버튼 클릭 | 즉시 | 입력 초기화 + 전체 정상 복귀 |
| 19 | 매칭 없는 쿼리 | 즉시 | 모든 노드 dim |
| 20 | 라이트/다크 테마 | 렌더 | dim 표현 색상 토큰 유지 |
| 21 | 노드 클릭 | 클릭 | 기존 뷰어 이동(`<a href>`) 유지 — 검색 상태와 독립 |
| 22 | 볼트 변경 (vault-changed) | 재로드 | 그래프 재로드 + 검색 상태 유지 (UX 판단) |

## Dependencies

- `src-tauri/src/models.rs` — `GraphNode.tags` 추가
- `src-tauri/src/commands/vault.rs` — `get_link_graph`에서 tags 채움
- `src/lib/types.ts` — GraphNode 타입 업데이트
- `src/lib/graph-filter.ts` — 신규 (순수 함수)
- `src/lib/components/LinkGraph.svelte` — `activeNodeIds` prop + opacity 렌더
- `src/routes/graph/+page.svelte` — 상단 바 + 상태 + `computeActiveIds` 호출

## Mock Boundary

- Rust: 기존 테스트 인프라에 1~2건 추가 (tags 반환 검증)
- 순수 함수 `graph-filter.ts`: 테스트 프레임워크 없음 → **수동 E2E**. 로직은 순수 함수로 추출되어 있어 추후 vitest 도입 시 즉시 테스트 가능.

## 테스트 목록 (Rust)

`vault.rs #[cfg(test)]` 또는 기존 테스트 인프라 재활용:

1. `link_graph_node_tags_from_frontmatter`
2. `link_graph_node_tags_empty_when_no_frontmatter`

(그래프 커맨드는 현재 vault.rs에 테스트가 없으므로, 기존 indexer 테스트 대상 구조를 쓰든가 생략하고 `models.rs` 구조체만 점검하는 얇은 테스트로 대체 가능)

## Edge Cases

- **매우 많은 태그/타입**: 드롭다운 성능 — `graph.nodes` 순회해 Set 구축, 정렬. 볼트 수천 노트도 문제 없음
- **쿼리 특수문자**: substring 매칭이므로 정규식 이스케이프 불필요
- **검색 중 볼트 전환**: `graph` 자체가 reload되므로 nodes/edges 갱신됨. 검색 상태는 유지하되 앵커가 재계산됨 — 일치하는 title이 새 볼트에 없으면 dim (22번 케이스)
- **tag 필터와 note_type null**: frontmatter 없으면 tags=[], note_type=null → type 필터 걸리면 전부 제외됨(의도대로)
- **노드 id에 특수문자**: id는 title 기반, 렌더 안정성은 기존대로 (변경 없음)

## Out of Scope (Slice 2.5)

- 클러스터 색상 분리 + 범례 (원래 A) — Epic 2 제외, 후속 버전 고려
- God Node 크기 스케일 (원래 B) — 동일
- 노드 클릭 포커스 모드 (원래 D) — 검색이 기능적으로 대체
- 품질 오버레이 (원래 E) — `AuditSummary` 컴포넌트로 충분
- 2-hop 이웃 토글 — 필요 시 후속 Slice
- 검색 결과 하이라이트 (텍스트 강조) — dim만으로 충분
- 키보드 단축키(`Cmd+F`) — v1.0 이후

## 열린 결정 (Slice 진입 전 확정)

- **스코프 C만** ← **확정** (2026-04-19 사용자 승인)
- **이웃 범위 1-hop** ← **확정**
- **비매칭 표현 dim (opacity 0.15)** ← **확정**
- **AND 조합** ← **확정**
- **title 검색만** ← **확정** (body는 Explore 페이지 몫)
- **필터 축 type + tag** ← **확정**
- **상단 고정 바** ← **확정**
- **노드 클릭 = 뷰어 이동(기존 유지)** ← **확정**
