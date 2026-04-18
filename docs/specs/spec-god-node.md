# Spec: god-node (v1.0 Epic 2.1 대시보드 God Node 카드)

## 전제

- Epic 1 완료 — 디자인 토큰(`bg-surface-1`, `border-border`, `--color-accent-fg`)과 카드 레이아웃이 대시보드에 안착
- 인덱서에 `VaultIndex.backlinks: HashMap<String, Vec<String>>` 존재 — **title 기준 역방향 링크 맵**
- `build_backlinks()`는 dedup된 source 목록을 제공. self-reference도 포함되어 있으므로 집계 시 별도 제외
- Graphify식 그래프 인텔리전스의 첫 산출물. "다른 노트가 많이 참조하는 노트 = 볼트의 핵심 허브"라는 가정
- God Node = **in-degree (backlink count)** 기준. outgoing은 포함하지 않음 (MOC/인덱스성 노트가 상위를 차지하지 않도록)

## Public Interface

### Rust — 집계 함수

```rust
// src-tauri/src/vault/indexer.rs
/// 볼트 인덱스에서 backlink 수 상위 N개 노트를 반환.
/// - incoming link(backlinks) 기준. outgoing은 무시.
/// - self-reference는 카운트에서 제외.
/// - broken wikilink(대상 노트가 실제로 없는 경우)는 결과에 포함하지 않음.
/// - 동률: note.path alphabetical 오름차순.
/// - backlink_count가 0인 노트는 포함하지 않음.
pub fn compute_top_god_nodes(index: &VaultIndex, limit: usize) -> Vec<GodNode>;
```

### Rust — 모델

```rust
// src-tauri/src/models.rs
#[derive(Debug, Clone, Serialize)]
pub struct GodNode {
    pub path: String,
    pub title: String,
    pub note_type: Option<NoteType>,
    pub backlink_count: usize,
}
```

### Tauri IPC 커맨드

```rust
// src-tauri/src/commands/vault.rs
#[tauri::command]
pub async fn get_top_god_nodes(
    state: State<'_, AppState>,
    limit: usize,
) -> Result<Vec<GodNode>, String>;
```

- 인덱스가 비어있거나 limit=0이면 빈 Vec 반환
- 활성 볼트가 설정되지 않은 경우는 기존 커맨드(`get_vault_stats`) 에러 규약을 따름

### TypeScript (frontend)

```ts
// src/lib/types.ts
export interface GodNode {
  path: string;
  title: string;
  note_type: NoteType | null;
  backlink_count: number;
}
```

### Svelte — 대시보드 카드

`src/routes/+page.svelte`에 God Node 카드 섹션 추가:

```
┌─ God Nodes ────────────────────────┐
│ 🔗 Rust Ownership          24 refs │
│ 🔗 Type Theory              18 refs│
│ 🔗 Monad                    12 refs│
│ 🔗 Category Theory           9 refs│
│ 🔗 Domain-Driven Design      7 refs│
└────────────────────────────────────┘
```

- 카드 제목: "핵심 노트" 또는 "God Nodes"
- 행 클릭 → 해당 노트 뷰어로 이동 (`/view?path={encodeURIComponent(path)}` — 기존 Recent Notes 컨벤션)
- `backlink_count` 뱃지: 우측 정렬, `text-fg-muted` 톤
- 상위 5개 고정 노출 (MVP에서 전개/확장 기능 없음)

## Invariants

- `compute_top_god_nodes(empty_index, 5) == vec![]`
- `compute_top_god_nodes(index, 0) == vec![]`
- `len(result) <= min(limit, notes_with_backlinks_count)`
- 반환 순서: `backlink_count` desc → `path` asc (결정론적)
- `backlink_count`는 dedup된 source 수와 동일 (기존 `build_backlinks` 계약)
- self-reference 제외: `note.path`가 backlinks[note.title]의 source 중 하나라면 -1
- broken link 제외: `index.backlinks[key]` 중 실제 `notes`에 존재하지 않는 title은 집계 대상 아님
- title 중복: 같은 title을 가진 노트가 여러 개라면 각각 별개 항목으로 반환 (backlinks가 title 기준 맵이므로 이 경우 양쪽이 동일 count를 공유 — 기존 인덱서 설계 결정 존중)

## Behavior Contract — Rust

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 빈 인덱스 (notes=[]) | `compute_top_god_nodes(&idx, 5)` | 빈 Vec |
| 2 | limit=0, 노트 3개 | `compute_top_god_nodes(&idx, 0)` | 빈 Vec |
| 3 | 노트 A, B, C; A는 B를 참조 | `compute_top_god_nodes(&idx, 5)` | `[B(1)]` (A/C는 backlink_count=0이라 제외) |
| 4 | A→B, C→B, D→E (B: 2, E: 1) | `compute_top_god_nodes(&idx, 5)` | `[B(2), E(1)]` (desc) |
| 5 | A→B, A→C (B: 1, C: 1) 동률, B.path="b.md" C.path="a.md" | `compute_top_god_nodes(&idx, 5)` | `[C(1), B(1)]` (path asc) |
| 6 | A→A (self-link), B→A (B: 0, A: 1) | `compute_top_god_nodes(&idx, 5)` | `[A(1)]` (self-ref 제외 → 1) |
| 7 | A→A (self-link), 다른 참조 없음 | `compute_top_god_nodes(&idx, 5)` | `[]` (self-ref 제외 → 0) |
| 8 | A→"Nonexistent" (broken link), A→B, C→B | `compute_top_god_nodes(&idx, 5)` | `[B(2)]` (broken link는 결과 제외) |
| 9 | 노트 10개, 모두 서로 1회씩 참조 (각각 backlink=9) | `compute_top_god_nodes(&idx, 5)` | 5개 반환, path asc 정렬 |
| 10 | A→A, A→B, C→B (A: 0 after self-ref, B: 2) | `compute_top_god_nodes(&idx, 5)` | `[B(2)]` |

## Behavior Contract — 프론트 (수동 검증)

| # | Given | When | Then |
|---|-------|------|------|
| 11 | 활성 볼트 로드 완료, 링크 있는 노트 존재 | 대시보드 진입 | God Node 섹션에 상위 5개 표시, 각 행에 title + backlink_count 뱃지 |
| 12 | God Node 행 클릭 | 클릭 | 해당 노트 뷰어(`/view?path=...`)로 이동 |
| 13 | 빈 볼트 또는 링크 없는 볼트 | 대시보드 진입 | God Node 섹션은 "아직 핵심 노트가 없습니다" 안내 표시 (또는 섹션 자체 미노출) |
| 14 | vault-changed 이벤트 수신 | 재로드 | vault.svelte.ts store 패턴 따라 God Node 카드도 자동 재로드 |
| 15 | 라이트/다크 테마 각각 | 렌더 | 카드 배경/텍스트/뱃지 색상이 각 테마 토큰 반영 |
| 16 | 컴팩트 밀도 토글 | 렌더 | 카드 내 간격이 밀도 토큰에 따라 축소 |

## Dependencies

- `src-tauri/src/models.rs` — `GodNode` 추가
- `src-tauri/src/vault/indexer.rs` — `compute_top_god_nodes` 추가
- `src-tauri/src/commands/vault.rs` — `get_top_god_nodes` 커맨드 추가
- `src-tauri/src/lib.rs` — `invoke_handler` 등록
- `src/lib/types.ts` — `GodNode` 타입 추가
- `src/routes/+page.svelte` — God Node 카드 섹션 추가
- `src/lib/stores/vault.svelte.ts` — vault-changed 이벤트 구독(이미 존재) 재사용

## Mock Boundary

- Rust: 순수 함수 테스트. 임시 `VaultIndex` 조립만으로 검증 가능 (파일 I/O 불필요)
- 프론트: 수동 E2E (`npm run tauri dev`) — 실제 볼트 대상

## 테스트 목록 (Rust 유닛)

`src-tauri/src/vault/indexer.rs #[cfg(test)]` 모듈에 추가:

1. `top_god_nodes_empty_index_returns_empty`
2. `top_god_nodes_zero_limit_returns_empty`
3. `top_god_nodes_excludes_notes_without_backlinks`
4. `top_god_nodes_orders_by_backlink_count_desc`
5. `top_god_nodes_ties_broken_by_path_asc`
6. `top_god_nodes_excludes_self_reference`
7. `top_god_nodes_self_only_reference_returns_empty`
8. `top_god_nodes_excludes_broken_links`
9. `top_god_nodes_respects_limit`

## Edge Cases

- **backlinks가 title 기준**: 같은 title을 가진 노트가 여러 개라면 backlinks[title]의 모든 source가 양쪽에 동일하게 카운트됨. 이는 기존 인덱서 설계 결정이며, God Node도 동일 계약 준수. title 중복은 볼트 품질 이슈이므로 Epic 2.3(볼트 헬스)에서 별도 경고 대상.
- **self-reference 검출 단위**: `backlinks[note.title]`의 source 중 `note.path`가 포함된 경우만 제외. 즉 title이 같은 다른 노트의 참조는 self가 아니다.
- **limit > 실제 후보 수**: 후보만큼만 반환
- **limit 비정상 큰 값**: `usize::MAX` 같은 값도 패닉 없이 처리 (반환 길이는 후보 수에 상한)
- **broken link**: `build_backlinks`는 실제 존재 여부와 무관하게 맵을 만든다. 집계 시 `notes`에 실제로 존재하는 title만 대상. 기존 `compute_stats`가 `note_titles` HashSet으로 broken link를 분리하는 패턴을 그대로 차용.

## Out of Scope (Slice 2.1)

- 클러스터/커뮤니티 탐지 → Slice 2.2
- 고아 노트/깨진 링크 카드 → Slice 2.3 (볼트 헬스)
- 최근 활동 타임라인 → Slice 2.4
- limit 설정 UI(설정에서 5 → N 변경) — MVP는 5 고정
- outgoing 허브(MOC 감지) — 별도 메트릭 필요
- 정규화 지표(노트 당 평균 backlink로 나눈 상대 점수) — 해석 복잡성 대비 이득 불명
- 시간 가중(최근 1개월 링크 가중치) — v1.0 이후 고려

## 열린 결정 (Slice 진입 전 확정)

- **"링크 수" 정의**: incoming-only(현 제안) ← **확정** (2026-04-18 사용자 승인)
- **동률 정렬**: path alphabetical(현 제안) ← **확정**
- **self-reference**: 제외(현 제안) ← **확정**
- **커맨드 형태**: `get_top_god_nodes(limit)` 신규(현 제안) ← **확정**
- **limit 기본값**: 5 (로드맵 명시)
- **카드 위치**: 대시보드 기존 4개 통계 카드 아래, 타입/상태 분포 위 (정보 위계상 핵심 지표 근처)
- **빈 상태 처리**: 안내 문구("아직 핵심 노트가 없습니다. 노트 간 링크를 추가하면 여기에 표시됩니다.") vs 섹션 숨김 → **안내 문구** 권장 (기능 발견성)
