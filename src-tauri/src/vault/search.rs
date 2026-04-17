use std::sync::{Arc, RwLock};

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, Value, STORED, STRING, TEXT};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};

use crate::error::AppError;
use crate::models::{NoteEntry, SearchResult, VaultIndex};

pub type SearchState = Arc<RwLock<SearchIndex>>;

pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    schema: Schema,
}

fn build_schema() -> Schema {
    let mut builder = Schema::builder();
    builder.add_text_field("path", STRING | STORED);
    builder.add_text_field("title", TEXT | STORED);
    builder.add_text_field("tags", TEXT | STORED);
    builder.add_text_field("body", TEXT);
    builder.build()
}

/// VaultIndex의 노트들로 tantivy 인메모리 인덱스 구축
pub fn build_search_index(notes: &[NoteEntry]) -> Result<SearchIndex, AppError> {
    let schema = build_schema();
    let index = Index::create_in_ram(schema.clone());

    let mut writer: IndexWriter = index
        .writer(15_000_000)
        .map_err(|e| AppError::Search(e.to_string()))?;

    let path_field = schema.get_field("path").unwrap();
    let title_field = schema.get_field("title").unwrap();
    let tags_field = schema.get_field("tags").unwrap();
    let body_field = schema.get_field("body").unwrap();

    for note in notes {
        let tags_text = note
            .frontmatter
            .as_ref()
            .map(|fm| fm.tags.join(" "))
            .unwrap_or_default();

        writer.add_document(doc!(
            path_field => note.path.as_str(),
            title_field => note.title.as_str(),
            tags_field => tags_text.as_str(),
            body_field => note.body.as_str(),
        )).map_err(|e| AppError::Search(e.to_string()))?;
    }

    writer.commit().map_err(|e| AppError::Search(e.to_string()))?;

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()
        .map_err(|e: tantivy::TantivyError| AppError::Search(e.to_string()))?;

    Ok(SearchIndex {
        index,
        reader,
        schema,
    })
}

/// 쿼리 실행 → SearchResult 반환
pub fn execute_search(
    search_index: &SearchIndex,
    vault_index: &VaultIndex,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, AppError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let title_field = search_index.schema.get_field("title").unwrap();
    let tags_field = search_index.schema.get_field("tags").unwrap();
    let body_field = search_index.schema.get_field("body").unwrap();
    let path_field = search_index.schema.get_field("path").unwrap();

    let query_parser = QueryParser::for_index(&search_index.index, vec![title_field, tags_field, body_field]);

    let tantivy_query = query_parser
        .parse_query(query)
        .map_err(|e| AppError::Search(e.to_string()))?;

    let searcher = search_index.reader.searcher();
    let top_docs = searcher
        .search(&tantivy_query, &TopDocs::with_limit(limit))
        .map_err(|e| AppError::Search(e.to_string()))?;

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for (_score, doc_address) in top_docs {
        let doc: TantivyDocument = searcher
            .doc(doc_address)
            .map_err(|e| AppError::Search(e.to_string()))?;

        let path = doc
            .get_first(path_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = doc
            .get_first(title_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tags_text = doc
            .get_first(tags_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // match_field 결정: title > tag > body
        let match_field = if title.to_lowercase().contains(&query_lower) {
            "title"
        } else if tags_text.to_lowercase().contains(&query_lower) {
            "tag"
        } else {
            "body"
        };

        // 본문 매칭 시 스니펫 추출
        let snippet = if match_field == "body" {
            let note = vault_index.notes.iter().find(|n| n.path == path);
            note.and_then(|n| extract_snippet(&n.body, &query_lower))
        } else {
            None
        };

        // VaultIndex에서 frontmatter, modified_at 가져오기
        let note = vault_index.notes.iter().find(|n| n.path == path);
        let (frontmatter, modified_at) = match note {
            Some(n) => (n.frontmatter.clone(), n.modified_at),
            None => (None, 0),
        };

        results.push(SearchResult {
            path,
            title,
            frontmatter,
            modified_at,
            snippet,
            match_field: match_field.to_string(),
        });
    }

    Ok(results)
}

/// 본문에서 쿼리 주변 컨텍스트를 추출하여 스니펫 생성
fn extract_snippet(body: &str, query_lower: &str) -> Option<String> {
    let body_lower = body.to_lowercase();
    let pos = body_lower.find(query_lower)?;

    let start = pos.saturating_sub(50);
    let end = (pos + query_lower.len() + 50).min(body.len());

    // UTF-8 경계 안전하게 조정
    let start = body.floor_char_boundary(start);
    let end = body.ceil_char_boundary(end);

    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(&body[start..end]);
    if end < body.len() {
        snippet.push_str("...");
    }

    Some(snippet)
}

/// 검색 인덱스 전체 재구축
#[allow(dead_code)]
pub fn rebuild_search_index(
    search_index: &mut SearchIndex,
    notes: &[NoteEntry],
) -> Result<(), AppError> {
    let new = build_search_index(notes)?;
    *search_index = new;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_note(path: &str, title: &str, body: &str, tags: Vec<&str>) -> NoteEntry {
        use crate::models::Frontmatter;
        use chrono::NaiveDate;
        use std::collections::HashMap;

        NoteEntry {
            path: path.to_string(),
            title: title.to_string(),
            frontmatter: Some(Frontmatter {
                note_type: crate::models::NoteType::Til,
                created: NaiveDate::from_ymd_opt(2026, 4, 16).unwrap(),
                tags: tags.into_iter().map(String::from).collect(),
                status: None,
                extra: HashMap::new(),
            }),
            outgoing_links: vec![],
            modified_at: 100,
            size: body.len() as u64,
            body: body.to_string(),
        }
    }

    fn make_index(notes: &[NoteEntry]) -> (SearchIndex, VaultIndex) {
        let vault_index = VaultIndex {
            notes: notes.to_vec(),
            backlinks: Default::default(),
            scanned_at: 0,
        };
        let search_index = build_search_index(notes).unwrap();
        (search_index, vault_index)
    }

    #[test]
    fn search_by_title() {
        let notes = vec![make_note("a.md", "Rust basics", "content here", vec!["dev"])];
        let (si, vi) = make_index(&notes);
        let results = execute_search(&si, &vi, "rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].match_field, "title");
    }

    #[test]
    fn search_by_tag() {
        let notes = vec![make_note("a.md", "my note", "some content", vec!["kotlin"])];
        let (si, vi) = make_index(&notes);
        let results = execute_search(&si, &vi, "kotlin", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].match_field, "tag");
    }

    #[test]
    fn search_by_body() {
        let notes = vec![make_note(
            "a.md",
            "my note",
            "This is about concurrency patterns in Go",
            vec!["dev"],
        )];
        let (si, vi) = make_index(&notes);
        let results = execute_search(&si, &vi, "concurrency", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].match_field, "body");
        assert!(results[0].snippet.is_some());
    }

    #[test]
    fn search_empty_query() {
        let notes = vec![make_note("a.md", "note", "body", vec![])];
        let (si, vi) = make_index(&notes);
        let results = execute_search(&si, &vi, "", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_no_match() {
        let notes = vec![make_note("a.md", "note", "body", vec![])];
        let (si, vi) = make_index(&notes);
        let results = execute_search(&si, &vi, "xyz999nonexistent", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_title_takes_priority_over_body() {
        let notes = vec![make_note(
            "a.md",
            "rust guide",
            "this note is about rust programming",
            vec![],
        )];
        let (si, vi) = make_index(&notes);
        let results = execute_search(&si, &vi, "rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].match_field, "title");
    }

    #[test]
    fn snippet_extraction() {
        let body = "The quick brown fox jumps over the lazy dog and finds a keyword hidden in the text somewhere around here";
        let snippet = extract_snippet(body, "keyword").unwrap();
        assert!(snippet.contains("keyword"));
    }

    #[test]
    fn snippet_at_start() {
        let body = "keyword is at the very beginning of this text";
        let snippet = extract_snippet(body, "keyword").unwrap();
        assert!(snippet.starts_with("keyword"));
    }

    #[test]
    fn snippet_not_found() {
        let body = "no match here";
        let result = extract_snippet(body, "xyz");
        assert!(result.is_none());
    }
}
