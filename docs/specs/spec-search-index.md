# Spec: Search Index (Slice E-1)

**상태**: Draft
**작성일**: 2026-04-27
**브랜치**: `feat/v2.0-epic-e-search`
**연관 로드맵**: Epic E / Slice E-1

## 목표

모든 등록 프로젝트의 (a) 마크다운 파일 본문, (b) graphify 노드 메타, (c) graphify 메모리 Q&A 를 단일 tantivy 인덱스에 색인한다.
이 슬라이스는 **인덱서 + 단위 테스트**만. 검색 IPC / UI / Cmd+K 는 후속.

## 데이터 모델

```rust
// src-tauri/src/search/mod.rs (신규)
pub mod index;
pub mod types;
```

```rust
// src-tauri/src/search/types.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind { File, Node, Qa }

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub project_id: String,
    pub project_name: String,
    pub kind: SearchKind,
    pub title: String,
    pub snippet: String,
    pub path: String,            // 파일 경로 또는 node id
    pub score: f32,
}
```

## 인덱스 스키마 (단일 tantivy 인덱스)

| 필드 | 타입 | 설명 |
|---|---|---|
| `project_id` | STRING (STORED \| INDEXED) | 프로젝트별 facet 필터 |
| `project_name` | STRING (STORED) | 결과 표시용 |
| `kind` | STRING (STORED \| INDEXED) | "file"/"node"/"qa" |
| `path` | STRING (STORED) | 파일 상대경로 또는 node id |
| `title` | TEXT (STORED) | 파일 제목 / 노드 라벨 / Q&A 제목 |
| `body` | TEXT | 파일 본문 / 노드 메타 요약 / Q&A 내용 |

- 인덱스 위치: `app_data_dir/search-index/` (영속). 시작 시 존재하면 그대로 사용, 없으면 새로 생성.
- 토크나이저는 기본 (영어 우선) — 한국어는 v2.1+ 에서 lindera 검토 (R6).
- BM25 기본.

## Public Interface (Slice E-1 한정)

```rust
// src-tauri/src/search/index.rs
pub struct SearchIndex {
    inner: tantivy::Index,
    schema: SearchSchema,
}

pub struct SearchSchema {
    pub project_id: tantivy::schema::Field,
    pub project_name: tantivy::schema::Field,
    pub kind: tantivy::schema::Field,
    pub path: tantivy::schema::Field,
    pub title: tantivy::schema::Field,
    pub body: tantivy::schema::Field,
}

/// 인덱스 디렉토리를 열거나 새로 만든다.
pub fn open_or_create(index_dir: &Path) -> Result<SearchIndex, SearchError>;

/// 한 프로젝트의 모든 문서를 재인덱싱한다 (기존 그 프로젝트 문서 삭제 후 삽입).
pub fn reindex_project(index: &SearchIndex, project: &ProjectEntry) -> Result<usize, SearchError>;

/// 모든 등록 프로젝트를 일괄 재인덱싱.
pub fn reindex_all(index: &SearchIndex, projects: &[ProjectEntry]) -> Result<usize, SearchError>;

/// 단순 검색 — Slice E-2 에서 IPC 로 노출. 여기서는 단위 테스트용.
pub fn search(
    index: &SearchIndex,
    query: &str,
    project_filter: Option<&[String]>,
    kind_filter: Option<&[SearchKind]>,
    limit: usize,
    project_name_lookup: &dyn Fn(&str) -> Option<String>,
) -> Result<Vec<SearchHit>, SearchError>;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("tantivy: {0}")]
    Tantivy(String),
    #[error("graphify: {0}")]
    Graphify(#[from] crate::graphify::reader::GraphifyError),
}
```

`SearchError::Graphify` 가 `NotRun` 인 경우는 reindex_project 가 (graphify 미실행) 무시하고 `.md` 파일만 인덱싱.

## reindex_project 알고리즘

1. 기존 인덱스에서 `project_id == project.id` 인 문서 모두 delete
2. **Files**: `docs_path` 재귀 walk → `.md` 파일 본문 (extract_frontmatter 로 title 추출, 첫 4 KiB 스캔), exclude_dirs/dotfile 제외
3. **Nodes**: `read_graphify_graph` 시도. 성공 시 각 노드를 (kind=Node, title=label, body=`norm_label + source_file + community + relation 요약`) 로 색인. NotRun 이면 스킵.
4. **Qa**: `graphify_out_path/memory/query_*.md` 파일들. 각각 (kind=Qa, title=파일명, body=본문 스캔). 디렉토리 없으면 스킵.
5. commit
6. 반환: 인덱싱된 문서 수

## Behavior Contract (단위 테스트 대상)

### `open_or_create`
- 빈 디렉토리 → 새 인덱스 생성
- 기존 인덱스 디렉토리 → 그대로 open
- 잘못된 디렉토리 (파일이면) → `Err(SearchError::Io)`

### `reindex_project` (TempDir 기반)
- docs/ 만 있고 graphify 없음 → `.md` 파일들만 색인됨, 노드/Qa 0
- docs/ + graph.json 있음 → 파일 + 노드 색인
- graph.json + memory/query_xxx.md 있음 → Qa 추가
- 같은 프로젝트 두 번 reindex → 문서 수 누적되지 않고 일정 (delete-then-insert)
- 다른 프로젝트 reindex → 첫 프로젝트 문서는 그대로 유지

### `search`
- 빈 인덱스 → 빈 결과
- 본문에 "JWT" 들어 있는 파일 색인 후 "JWT" 검색 → 결과에 포함
- project_filter 적용 → 다른 프로젝트 결과 제외
- kind_filter 적용 → 다른 kind 제외
- limit → 결과 개수 제한

## Frontend (E-1 한정)

이번 슬라이스는 IPC 노출 안 함. 후속 E-2 에서 `search` 커맨드 + `/search` 페이지.

## Dependencies

- `tantivy` 크레이트 추가 (이전 v1 시절 search.rs 가 사용했지만 그때 삭제됨 — 다시 추가)
- `cang-jie` / `lindera` 는 v2.1+ 검토 (이번 슬라이스 기본 토크나이저)

## 비범위

- 검색 IPC / `/search` UI → E-2/E-3
- Cmd+K 팔레트 → E-4
- 한국어 토크나이저
- 자동 재인덱싱 (graphify-updated 이벤트 trigger) → E-2 또는 별도

## 작업 순서 (TDD)

1. `tantivy` 의존성 추가
2. `src-tauri/src/search/{mod,types,index}.rs` 골격 + SearchError
3. SearchSchema + open_or_create + 단위 테스트
4. reindex_project 단위 테스트 (TempDir 기반 픽스처) → 구현
5. search 단위 테스트 → 구현
6. lib.rs 에 `mod search;` 추가 (#[allow(dead_code)] until E-2)
7. cargo test + clippy
8. 단일 커밋: `feat: tantivy 통합 검색 인덱서 (Slice E-1)`

## 완료 조건

- 인덱스 open/create 가능
- 단일/다중 프로젝트 reindex 가능
- 검색 결과 BM25 score + project_name 포함
- 기본 BC 8개 이상 green
- IPC 노출 없음 (E-2)
