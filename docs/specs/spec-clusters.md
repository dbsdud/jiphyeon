# Spec: clusters (v1.0 Epic 2.2 대시보드 클러스터 요약 카드)

## 전제

- Slice 2.1(God Node) 완료 — `backlinks: HashMap<String, Vec<String>>`, 대표 노트 선정 규칙(backlink desc → path asc) 확립
- Epic 2 전체 범위 중 **대시보드 카드**만 Slice 2.2의 스코프. 그래프 페이지 색상/범례/포커스는 **Slice 2.5로 이관**
- 클러스터 = 무방향 그래프의 연결 컴포넌트(Connected Component, 이하 CC). 링크의 방향을 무시하고 양방향 엣지로 취급
- 대시보드 카드는 "볼트가 얼마나 연결되어 있나?"의 한눈 파악용 — 상태 요약에 집중, 시각화는 그래프 페이지로 위임
- 대시보드 카드에서 그래프 페이지로 `/graph` **단순 링크**만 추가 (쿼리 파라미터 없음 — 2.5에서 확장)

## Public Interface

### Rust — 집계 함수

```rust
// src-tauri/src/vault/indexer.rs
/// 볼트의 연결 컴포넌트 요약을 반환.
/// - 무방향 그래프로 취급 (A→B 링크를 A-B 양방향 엣지로 간주)
/// - broken link(대상 노트가 존재하지 않는 경우)는 엣지에서 제외
/// - 크기 1(고립) 컴포넌트는 `clusters`에 포함하지 않고 `isolated_count`로 분리 집계
/// - `clusters`는 size desc → 대표 노트 path asc로 정렬
/// - 각 `ClusterInfo`의 대표 노트: backlink_count desc → path asc (God Node 규칙 재사용)
pub fn compute_clusters(index: &VaultIndex) -> ClusterSummary;
```

### Rust — 모델

```rust
// src-tauri/src/models.rs
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ClusterInfo {
    /// 안정적 ID. 결정론적으로 부여: 정렬 후 인덱스(0, 1, 2, ...)
    pub id: usize,
    /// 컴포넌트 내 노트 수 (>= 2 — 고립 제외)
    pub size: usize,
    /// 컴포넌트 대표 노트 (backlink_count desc → path asc 1위)
    pub representative_path: String,
    pub representative_title: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ClusterSummary {
    /// 전체 클러스터 수 (크기 >= 2). 고립 노트는 제외.
    pub cluster_count: usize,
    /// 전체 노트 중 가장 큰 클러스터의 노트 수
    pub largest_size: usize,
    /// 크기 1 연결 컴포넌트 개수 (= 고립 노트 수). 참고용.
    pub isolated_count: usize,
    /// 크기 >= 2 클러스터 목록. size desc → representative_path asc 정렬.
    pub clusters: Vec<ClusterInfo>,
}
```

### Tauri IPC 커맨드

```rust
// src-tauri/src/commands/vault.rs
#[tauri::command]
pub fn get_cluster_summary(
    state: State<'_, VaultState>,
) -> Result<ClusterSummary, AppError>;
```

- 인덱스가 비어있으면 `ClusterSummary { cluster_count: 0, largest_size: 0, isolated_count: 0, clusters: vec![] }` 반환
- 활성 볼트 미설정 시 기존 커맨드 에러 규약(`AppError::VaultNotFound` 등) 준수

### TypeScript

```ts
// src/lib/types.ts
export interface ClusterInfo {
  id: number;
  size: number;
  representative_path: string;
  representative_title: string;
}

export interface ClusterSummary {
  cluster_count: number;
  largest_size: number;
  isolated_count: number;
  clusters: ClusterInfo[];
}
```

### API wrapper

```ts
// src/lib/api.ts
export function getClusterSummary(): Promise<ClusterSummary> {
  return invoke("get_cluster_summary");
}
```

### Svelte — 대시보드 카드

`src/routes/+page.svelte`에 God Node 카드 아래에 클러스터 카드 섹션 추가:

```
┌─ Clusters ────────────────────────────────┐
│ 클러스터 3개 · 최대 42개(87%) · 고립 5개     │
│                                           │
│ 🌐 Rust Ownership          42 notes       │
│ 🌐 Domain-Driven Design     5 notes       │
│ 🌐 Meeting Notes            3 notes       │
│                                           │
│                         그래프 보기 →       │
└───────────────────────────────────────────┘
```

- 헤더 라인: 요약 텍스트 (`cluster_count`, `largest_size`, `largest_size/total_notes` 백분율, `isolated_count`)
- 본문: 상위 3개 클러스터 (크기 desc → 대표 노트 path asc)
- 각 행: 대표 노트 title + "N notes" 뱃지
- 행 클릭 → 대표 노트 뷰어로 이동 (`/view?path=...`)
- 카드 하단 우측에 **그래프 보기 →** 링크 (`href="/graph"`)
- 빈 상태(cluster_count=0): 안내 문구 "아직 연결된 노트 그룹이 없습니다."

## Invariants

- `largest_size >= 2` when `cluster_count > 0`, else `largest_size == 0`
- `cluster_count == clusters.len()`
- `isolated_count + sum(c.size for c in clusters) == total_notes` (모든 노트는 정확히 하나의 CC에 속함)
- `clusters`는 size desc → representative_path asc (결정론적)
- `ClusterInfo.id`는 정렬 후 0부터 부여 (id=0이 최대 클러스터)
- broken link는 엣지로 반영되지 않음 — wikilink의 target이 실제 노트가 아니면 무시
- self-link(A→A)는 엣지에 영향 없음 (이미 같은 CC)
- backlinks 맵이 title 기준이라 title 중복 시 CC가 합쳐질 수 있음 — 기존 인덱서 결정 존중 (Slice 2.3 볼트 헬스에서 경고)

## Behavior Contract — Rust

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 빈 인덱스 | `compute_clusters(&idx)` | `cluster_count=0, largest_size=0, isolated_count=0, clusters=[]` |
| 2 | 노트 3개, 링크 없음 | `compute_clusters(&idx)` | `cluster_count=0, isolated_count=3, clusters=[]` |
| 3 | A→B, C (고립) | `compute_clusters(&idx)` | `cluster_count=1, largest_size=2, isolated_count=1, clusters=[{size:2, rep: A 또는 B}]` |
| 4 | A→B→C (체인) | `compute_clusters(&idx)` | `cluster_count=1, largest_size=3, isolated_count=0` |
| 5 | A→B, C→D (두 그룹) | `compute_clusters(&idx)` | `cluster_count=2, largest_size=2, isolated_count=0` |
| 6 | A→B, C→D, E→F, G→H, 4개 그룹 각각 size=2 | `compute_clusters(&idx)` | `cluster_count=4, largest_size=2`, clusters는 representative_path asc |
| 7 | A→B, A→Nonexistent (broken) | `compute_clusters(&idx)` | broken link 무시 — `{A,B}`만 한 CC, `cluster_count=1, size=2` |
| 8 | A→A (self-link), B (고립) | `compute_clusters(&idx)` | `cluster_count=0, isolated_count=2` (A의 self-link는 CC 크기를 키우지 않음) |
| 9 | A→B, B의 backlink_count=1, A의 backlink_count=0 | `compute_clusters(&idx)` | 대표 노트는 B (backlink desc) |
| 10 | A→B, 둘 다 backlink_count=1 (서로 참조) | `compute_clusters(&idx)` | 대표 노트는 path asc로 결정 (동률 tie-break) |
| 11 | 큰 컴포넌트(5노트) + 작은 컴포넌트(2노트) | `compute_clusters(&idx)` | clusters[0].size=5, clusters[1].size=2 (size desc) |
| 12 | 동일 size의 두 CC, 대표 path가 "a.md"와 "b.md" | `compute_clusters(&idx)` | clusters[0].representative_path="a.md" (path asc) |

## Behavior Contract — 프론트 (수동 검증)

| # | Given | When | Then |
|---|-------|------|------|
| 13 | 링크 있는 볼트 | 대시보드 진입 | God Node 카드 아래에 Clusters 카드 표시, 요약 라인 + 상위 3개 노출 |
| 14 | 클러스터 카드의 대표 노트 행 클릭 | 클릭 | `/view?path=<path>`로 이동 |
| 15 | 카드 하단 "그래프 보기 →" 클릭 | 클릭 | `/graph` 이동 (쿼리 파라미터 없음) |
| 16 | 빈 볼트 또는 링크 없는 볼트 | 대시보드 진입 | 안내 문구 "아직 연결된 노트 그룹이 없습니다." |
| 17 | 단일 클러스터 (볼트 전체가 하나의 CC) | 대시보드 진입 | 요약에 "클러스터 1개 · 최대 N개(100%) · 고립 0개", 상위 1개만 표시 |
| 18 | 클러스터 3개, 각 size=2 | 대시보드 진입 | 상위 3개 모두 표시 (4개 이상이어도 상위 3개만) |
| 19 | vault-changed 이벤트 | 재로드 | 클러스터 카드도 갱신 |
| 20 | 라이트/다크 테마 | 렌더 | 카드 색상 토큰 반영 |
| 21 | compact 밀도 | 렌더 | 카드 내 간격 축소 |

## Dependencies

- `src-tauri/src/models.rs` — `ClusterInfo`, `ClusterSummary` 추가
- `src-tauri/src/vault/indexer.rs` — `compute_clusters` 추가
- `src-tauri/src/commands/vault.rs` — `get_cluster_summary` 커맨드
- `src-tauri/src/lib.rs` — `invoke_handler` 등록
- `src/lib/types.ts` — 타입 추가
- `src/lib/api.ts` — `getClusterSummary` 추가
- `src/routes/+page.svelte` — 클러스터 카드 섹션

## Mock Boundary

- Rust: 순수 함수. `NoteEntry` 목록으로 `VaultIndex` 조립 → 테스트 (파일 I/O 없음)
- 프론트: 수동 E2E

## 알고리즘

**무방향 그래프 CC 탐색 (BFS)**

```
1. 노드 집합 = index.notes (title 기준)
2. 엣지 집합 = {(note.title, target) for note in notes for target in note.outgoing_links
                if target in note_titles and target != note.title}
   — broken link와 self-link는 엣지에서 제외
3. 인접 리스트 구축 (양방향):
   for (u, v) in edges:
     adj[u].insert(v); adj[v].insert(u)
4. visited = {}
5. components = []
6. for title in sorted(notes by path):
     if title not in visited:
       bfs로 current component 수집
       components.push(component)
7. components를 size desc → representative path asc로 정렬
8. 각 component의 representative = backlink_count desc → path asc
9. 크기 1은 isolated_count, 크기 >= 2는 clusters로 분리
```

복잡도: O(V + E). 수백~수천 노트 볼트에서 밀리초 수준.

**title 기준 CC → path 기반 결과 매핑**: adjacency는 title로 구축하되, 결과 `ClusterInfo`는 path/title을 담아 반환. 같은 title을 가진 여러 노트는 같은 CC에 합쳐짐(기존 인덱서 설계 한계).

## 테스트 목록 (Rust 유닛)

`src-tauri/src/vault/indexer.rs #[cfg(test)]` 모듈에 추가:

1. `clusters_empty_index_returns_zero`
2. `clusters_no_links_returns_isolated_only`
3. `clusters_single_edge_makes_pair`
4. `clusters_chain_merges_into_one`
5. `clusters_two_separate_groups`
6. `clusters_many_equal_size_sorted_by_path`
7. `clusters_ignores_broken_links`
8. `clusters_self_link_does_not_create_cluster`
9. `clusters_representative_by_backlink_count`
10. `clusters_representative_ties_broken_by_path`
11. `clusters_sorted_by_size_desc`
12. `clusters_same_size_tiebreak_by_representative_path`

## Edge Cases

- **title 중복**: backlinks 맵이 title 기준 → 같은 title을 가진 두 노트는 같은 그룹으로 합쳐짐. Slice 2.3의 볼트 헬스에서 경고 예정. 이 Spec에서는 별도 처리 없음.
- **대형 볼트**: V+E 선형 복잡도. 10k 노트도 문제 없음. GC 압박 최소화 위해 HashSet 대신 HashMap + 인덱스 사용 검토 가능하나 YAGNI.
- **self-link만 있는 노트**: broken link도, self-link도 엣지에서 제외되므로 고립으로 집계됨 (Edge case #8 테스트).
- **broken link 집계**: `compute_stats`가 이미 `broken_links` 노출 — 클러스터 집계에서는 엣지 제외로 일관성 유지.
- **상위 3개 미만**: 클러스터가 0/1/2개여도 UI가 깨지지 않도록 안전하게 렌더.

## Out of Scope (Slice 2.2)

- 그래프 페이지 클러스터 색상 분리 → **Slice 2.5**
- 클러스터별 필터링 (`/graph?cluster=<id>`) → **Slice 2.5**
- 클러스터 전체 목록 페이지 (대시보드는 상위 3개만) — MVP에서 불요
- 가중치 기반 커뮤니티 탐지(Louvain 등) — CC로 충분
- 고립 노트 상세 목록 — **Slice 2.3** 볼트 헬스에서 제공
- 대형 볼트 성능 최적화 — 필요 시 별도

## 열린 결정 (Slice 진입 전 확정)

- **엣지 방향성**: 무방향 ← **확정** (2026-04-18 사용자 승인)
- **알고리즘**: BFS ← **확정**
- **broken link**: 엣지 제외 ← **확정**
- **대표 노트 선정**: backlink desc → path asc ← **확정** (God Node 규칙 재사용)
- **대시보드 UI**: 요약 + 상위 3개 + `/graph` 단순 링크 ← **확정**
- **고립 노트**: clusters에서 제외, isolated_count로 분리 집계 ← **확정**
- **카드 위치**: God Node 카드 바로 아래 ← 권장 (정보 위계: 핵심 노트 → 연결 구조)
- **노출 개수**: 상위 3 (MVP 고정). 더 많은 클러스터 노출은 2.5에서 그래프로 위임.
