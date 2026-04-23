# Spec: full-text-search (Phase 3)

## Public Interface

### Backend

```rust
/// 전문 검색 — 제목, 태그, 본문을 tantivy로 검색
#[tauri::command]
pub fn search_notes(
    state: State<'_, VaultState>,
    search_state: State<'_, SearchState>,
    query: String,
) -> Result<Vec<SearchResult>, AppError>
```

기존 `search_notes` 시그니처 변경: 반환 타입 `Vec<NoteEntry>` → `Vec<SearchResult>`.

### 새 타입

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub frontmatter: Option<Frontmatter>,
    pub modified_at: i64,
    pub snippet: Option<String>,    // 매칭 컨텍스트 (~120자, "...앞문맥 **쿼리** 뒷문맥...")
    pub match_field: String,        // "title" | "tag" | "body"
}
```

### 검색 인덱스 모듈

```rust
// vault/search.rs (신규)

pub type SearchState = Arc<RwLock<SearchIndex>>;

pub struct SearchIndex {
    index: tantivy::Index,
    reader: tantivy::IndexReader,
}

/// VaultIndex로부터 tantivy 인덱스 구축
pub fn build_search_index(notes: &[NoteEntry]) -> Result<SearchIndex, AppError>

/// 쿼리 실행 → SearchResult 반환
pub fn execute_search(
    search_index: &SearchIndex,
    vault_index: &VaultIndex,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, AppError>

/// 단일 노트 인덱스 업데이트 (incremental)
pub fn update_note(search_index: &mut SearchIndex, note: &NoteEntry) -> Result<(), AppError>

/// 단일 노트 인덱스 삭제
pub fn remove_note(search_index: &mut SearchIndex, path: &str) -> Result<(), AppError>
```

### NoteEntry 변경

```rust
pub struct NoteEntry {
    // 기존 필드 유지
    pub path: String,
    pub title: String,
    pub frontmatter: Option<Frontmatter>,
    pub outgoing_links: Vec<String>,
    pub modified_at: i64,
    pub size: u64,
    // 추가
    #[serde(skip)]
    pub body: String,  // frontmatter 제거된 plain text (프론트엔드 전송 제외)
}
```

### Frontend

```typescript
// types.ts
export interface SearchResult {
  path: string;
  title: string;
  frontmatter?: Frontmatter;
  modified_at: number;
  snippet?: string;
  match_field: string;
}

// api.ts
export function searchNotes(query: string): Promise<SearchResult[]>
```

## Behavior Contract

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 제목에 "rust" 포함 노트 | search("rust") | 해당 노트 반환, match_field="title" |
| 2 | 태그에 "rust" 포함 노트 | search("rust") | 해당 노트 반환, match_field="tag" |
| 3 | 본문에만 "rust" 포함 노트 | search("rust") | 해당 노트 반환, match_field="body", snippet 포함 |
| 4 | 제목+본문 동시 매칭 | search("rust") | 하나의 결과로 반환, match_field="title" (우선) |
| 5 | 매칭 없음 | search("xyz999") | 빈 Vec |
| 6 | 빈 쿼리 | search("") | 빈 Vec |
| 7 | 본문 매칭 스니펫 | search("keyword") | "...앞50자 keyword 뒤50자..." 형태 |
| 8 | 한국어 검색 | search("개발") | NgramTokenizer로 매칭 |
| 9 | 노트 수정 후 재검색 | 파일 변경 → watcher → reindex | 변경된 내용 반영 |

## Edge Cases

- 1자 쿼리 → tantivy 기본 처리 (ngram 최소 단위에 따라)
- 특수문자 쿼리 (`[[`, `#`) → 이스케이프 처리
- frontmatter만 있고 본문 없는 노트 → body="" 인덱싱, 검색 대상 제외 아님
- 1,000개 노트 인덱싱 성능 → 인메모리 인덱스, < 1초 목표

## Dependencies

- `tantivy = "0.22"` — 전문 검색 엔진
- Mock boundary: 없음 (순수 인메모리 인덱싱)
- 기존 의존: `parser::parse_note` (body 텍스트 추출)

## tantivy 스키마

```rust
let mut schema_builder = Schema::builder();
schema_builder.add_text_field("path", STRING | STORED);
schema_builder.add_text_field("title", TEXT | STORED);
schema_builder.add_text_field("tags", TEXT);
schema_builder.add_text_field("body", TEXT);
```

## 인덱스 라이프사이클

1. 앱 시작 → `scan_vault()` → `build_search_index(notes)` → `SearchState`로 등록
2. 파일 변경 → watcher → incremental `update_note()` / `remove_note()`
3. `rescan_vault()` → 전체 재구축
