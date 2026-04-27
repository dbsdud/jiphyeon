# Spec: Pending Graphify Badge (Slice D-4)

**상태**: Draft
**작성일**: 2026-04-26
**선행**: Epic C 전체, D-3
**연관 로드맵**: Epic D / Slice D-6 (재명명: D-4)

## 목표

활성 프로젝트의 docs/ 가 graphify 실행 시점 이후로 변경되었는지 한눈에 보여주는 상태 뱃지를 사이드바에 추가. 사용자가 capture 후 graphify 재실행 필요성을 즉시 인지.

## 상태 분류

| 상태 | 조건 | 표시 |
|---|---|---|
| ✅ 최신 | `graph.json mtime ≥ docs/ 최신 mtime` | 초록 점 + "최신" |
| ⚠️ 변경됨 | `graph.json mtime < docs/ 최신 mtime` | 주황 점 + "N개 파일 변경" |
| ❌ 미실행 | graph.json 없음 | 회색 점 + "graphify 미실행" |
| — | 활성 프로젝트 없음 | 표시 없음 |

## Backend

### 신규 IPC

```rust
#[tauri::command]
pub fn get_pending_graphify(state: State<'_, ConfigState>) -> Result<PendingGraphify, AppError>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PendingGraphify {
    pub project_id: Option<String>,
    pub status: PendingStatus, // "fresh" | "stale" | "not_run" | "no_project"
    pub graph_run_at: Option<i64>,    // unix seconds
    pub docs_changed_at: Option<i64>, // unix seconds (가장 최근 .md mtime)
    pub changed_files_count: usize,   // graph_run_at 이후 수정된 .md 개수
}
```

### 알고리즘 (`compute_pending_graphify`)

1. 활성 프로젝트 없음 → `no_project`
2. `graphify-out/graph.json` 없음 → `not_run`
3. graph.json mtime = `graph_run_at`
4. `docs_path` 재귀 walk:
   - `.md` 파일만 (확장자 매칭)
   - exclude_dirs + dotfile 폴더 제외 (기존 `DEFAULT_EXCLUDE_TREE_DIRS` + `AppConfig.exclude_dirs`)
   - 각 파일 mtime 수집
5. `docs_changed_at` = max(mtime)
6. `changed_files_count` = `mtime > graph_run_at` 인 파일 수
7. status = `changed_files_count > 0 ? "stale" : "fresh"`

성능: 활성 프로젝트의 docs 만 walk. 수백 ~ 수천 파일 가정 (현재 list_project_files 와 동일 정책).

## Frontend

### 사이드바 표시

- 활성 프로젝트 항목 옆에 작은 점 + tooltip
  - `●` (success/warning/muted) + hover 시 메시지
- 매번 polling 대신 `vaultRefresh.version` 추적해 자동 재계산
  - 그래프 갱신 직후 `graphify-updated` 도 같이 트리거 → tactic은 vaultRefresh.bump 와 동일

### IPC 사용

- `+layout.svelte`: `getPendingGraphify()` 를 활성 프로젝트 영역에서 호출
- 반환 값으로 dot 색 + tooltip 메시지 결정

## Behavior Contract

### `compute_pending_graphify`

- Given: project=None
- Then: status="no_project", 카운트 0
- Given: project, graph.json 없음
- Then: status="not_run", graph_run_at=None
- Given: project, graph.json 있음, docs/ 빈 폴더
- Then: status="fresh", changed_files_count=0
- Given: graph.json mtime=T1, docs/foo.md mtime=T1-10
- Then: status="fresh"
- Given: graph.json mtime=T1, docs/foo.md mtime=T1+10
- Then: status="stale", changed_files_count=1
- Given: 두 파일 모두 mtime>T1
- Then: changed_files_count=2

## 비범위

- docs 외 graphify-out 의 다른 파일들 변경 추적 (graph.json 자체만)
- "graphify 재실행" 버튼 (D-?, v2.5+)
- 다른 프로젝트의 pending 상태 표시 (활성만)

## 작업 순서

1. backend: `commands/graphify.rs` 또는 신규 `commands/pending.rs` — `compute_pending_graphify` pure fn + 6 BC + IPC
2. lib.rs invoke_handler 등록
3. frontend types.ts: PendingGraphify, PendingStatus
4. api.ts: getPendingGraphify
5. +layout.svelte: 활성 프로젝트 행 옆에 점 표시 + tooltip
6. cargo test + clippy + svelte-check
7. 커밋: `feat: pending graphify 뱃지 (Slice D-4)`

## 완료 조건

- 활성 프로젝트 사이드바 항목 옆에 색깔 점 표시
- 6 BC green
- 모든 검증 green
