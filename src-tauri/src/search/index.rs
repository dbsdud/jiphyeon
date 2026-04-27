//! 통합 검색 인덱스 (tantivy 단일 인덱스, kind 필드로 file/node/qa 구분).

use std::fs;
use std::path::{Path, PathBuf};

use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, SchemaBuilder, Value, STORED, STRING, TEXT,
};
use tantivy::{
    directory::MmapDirectory, doc, Index, IndexReader, ReloadPolicy, TantivyDocument, Term,
};

use crate::graphify::reader::{read_graphify_graph, GraphifyError};
use crate::project::ProjectEntry;
use crate::search::types::{SearchHit, SearchKind};
use crate::vault::parser::extract_frontmatter;

const TITLE_SCAN_BYTES: usize = 4096;
const DEFAULT_EXCLUDE_DIRS: &[&str] = &[".git", ".claude", "node_modules", "target", "graphify-out"];
const SNIPPET_LEN: usize = 160;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("tantivy: {0}")]
    Tantivy(String),
    #[error("graphify: {0}")]
    Graphify(#[from] GraphifyError),
}

impl serde::Serialize for SearchError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<tantivy::TantivyError> for SearchError {
    fn from(e: tantivy::TantivyError) -> Self {
        SearchError::Tantivy(e.to_string())
    }
}

impl From<tantivy::directory::error::OpenDirectoryError> for SearchError {
    fn from(e: tantivy::directory::error::OpenDirectoryError) -> Self {
        SearchError::Tantivy(e.to_string())
    }
}

#[derive(Clone)]
pub struct SearchSchema {
    pub project_id: Field,
    pub project_name: Field,
    pub kind: Field,
    pub path: Field,
    pub title: Field,
    pub body: Field,
}

pub struct SearchIndex {
    inner: Index,
    schema: SearchSchema,
    reader: IndexReader,
}

// SearchIndex 의 내부 필드는 같은 모듈 안의 함수들이 직접 사용.

fn build_schema() -> (Schema, SearchSchema) {
    let mut b = SchemaBuilder::new();
    let project_id = b.add_text_field("project_id", STRING | STORED);
    let project_name = b.add_text_field("project_name", STRING | STORED);
    let kind = b.add_text_field("kind", STRING | STORED);
    let path = b.add_text_field("path", STRING | STORED);
    let title = b.add_text_field("title", TEXT | STORED);
    let body = b.add_text_field("body", TEXT | STORED);
    let schema = b.build();
    (
        schema,
        SearchSchema {
            project_id,
            project_name,
            kind,
            path,
            title,
            body,
        },
    )
}

pub fn open_or_create(index_dir: &Path) -> Result<SearchIndex, SearchError> {
    if index_dir.exists() && !index_dir.is_dir() {
        return Err(SearchError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("index path is not a directory: {}", index_dir.display()),
        )));
    }
    fs::create_dir_all(index_dir)?;
    let (schema, fields) = build_schema();
    let dir = MmapDirectory::open(index_dir)?;
    let inner = Index::open_or_create(dir, schema)?;
    let reader = inner
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into()?;
    Ok(SearchIndex {
        inner,
        schema: fields,
        reader,
    })
}

/// 한 프로젝트의 모든 문서를 재인덱싱한다.
pub fn reindex_project(
    index: &SearchIndex,
    project: &ProjectEntry,
) -> Result<usize, SearchError> {
    let mut writer = index.inner.writer(15_000_000)?;
    let term = Term::from_field_text(index.schema.project_id, &project.id);
    writer.delete_term(term);

    let mut count = 0usize;

    // Files
    if project.docs_path.is_dir() {
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        collect_md(&project.docs_path, &project.docs_path, &mut files);
        for (rel, body) in files {
            let title = title_from_path_or_frontmatter(&body, &rel);
            writer.add_document(doc!(
                index.schema.project_id => project.id.as_str(),
                index.schema.project_name => project.name.as_str(),
                index.schema.kind => SearchKind::File.as_index_str(),
                index.schema.path => rel.to_string_lossy().to_string(),
                index.schema.title => title.as_str(),
                index.schema.body => body.as_str(),
            ))?;
            count += 1;
        }
    }

    // Nodes
    match read_graphify_graph(&project.graphify_out_path) {
        Ok(g) => {
            for n in g.nodes {
                let mut body = String::new();
                if let Some(s) = &n.norm_label {
                    body.push_str(s);
                    body.push('\n');
                }
                if let Some(s) = &n.source_file {
                    body.push_str(s);
                    body.push('\n');
                }
                if let Some(s) = &n.file_type {
                    body.push_str(s);
                    body.push('\n');
                }
                if let Some(c) = n.community {
                    body.push_str(&format!("community {}\n", c));
                }
                writer.add_document(doc!(
                    index.schema.project_id => project.id.as_str(),
                    index.schema.project_name => project.name.as_str(),
                    index.schema.kind => SearchKind::Node.as_index_str(),
                    index.schema.path => n.id.as_str(),
                    index.schema.title => n.label.as_str(),
                    index.schema.body => body.as_str(),
                ))?;
                count += 1;
            }
        }
        Err(GraphifyError::NotRun) => {}
        Err(e) => return Err(SearchError::Graphify(e)),
    }

    // Qa
    let memory_dir = project.graphify_out_path.join("memory");
    if memory_dir.is_dir() {
        if let Ok(read) = fs::read_dir(&memory_dir) {
            for entry in read.flatten() {
                let p = entry.path();
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else { continue };
                if !name.starts_with("query_") || !name.ends_with(".md") {
                    continue;
                }
                let body = fs::read_to_string(&p).unwrap_or_default();
                let title = name.trim_end_matches(".md").to_string();
                writer.add_document(doc!(
                    index.schema.project_id => project.id.as_str(),
                    index.schema.project_name => project.name.as_str(),
                    index.schema.kind => SearchKind::Qa.as_index_str(),
                    index.schema.path => name,
                    index.schema.title => title.as_str(),
                    index.schema.body => body.as_str(),
                ))?;
                count += 1;
            }
        }
    }

    writer.commit()?;
    index.reader.reload()?;
    Ok(count)
}

pub fn reindex_all(
    index: &SearchIndex,
    projects: &[ProjectEntry],
) -> Result<usize, SearchError> {
    let mut total = 0;
    for p in projects {
        total += reindex_project(index, p)?;
    }
    Ok(total)
}

pub fn search(
    index: &SearchIndex,
    query: &str,
    project_filter: Option<&[String]>,
    kind_filter: Option<&[SearchKind]>,
    limit: usize,
) -> Result<Vec<SearchHit>, SearchError> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let qp = QueryParser::for_index(&index.inner, vec![index.schema.title, index.schema.body]);
    let user_query = qp
        .parse_query(query)
        .map_err(|e| SearchError::Tantivy(e.to_string()))?;

    let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, user_query)];

    if let Some(ids) = project_filter {
        if !ids.is_empty() {
            let inner: Vec<(Occur, Box<dyn Query>)> = ids
                .iter()
                .map(|id| {
                    let term = Term::from_field_text(index.schema.project_id, id);
                    let q: Box<dyn Query> =
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic));
                    (Occur::Should, q)
                })
                .collect();
            clauses.push((Occur::Must, Box::new(BooleanQuery::new(inner))));
        }
    }

    if let Some(kinds) = kind_filter {
        if !kinds.is_empty() {
            let inner: Vec<(Occur, Box<dyn Query>)> = kinds
                .iter()
                .map(|k| {
                    let term = Term::from_field_text(index.schema.kind, k.as_index_str());
                    let q: Box<dyn Query> =
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic));
                    (Occur::Should, q)
                })
                .collect();
            clauses.push((Occur::Must, Box::new(BooleanQuery::new(inner))));
        }
    }

    let final_query = BooleanQuery::new(clauses);
    let searcher = index.reader.searcher();
    let top = searcher.search(&final_query, &TopDocs::with_limit(limit))?;

    let q_lower = query.to_lowercase();
    let mut hits = Vec::with_capacity(top.len());
    for (score, addr) in top {
        let doc: TantivyDocument = searcher.doc(addr)?;
        let project_id = field_str(&doc, index.schema.project_id).unwrap_or_default();
        let project_name = field_str(&doc, index.schema.project_name).unwrap_or_default();
        let kind_str = field_str(&doc, index.schema.kind).unwrap_or_default();
        let kind = SearchKind::from_index_str(&kind_str).unwrap_or(SearchKind::File);
        let title = field_str(&doc, index.schema.title).unwrap_or_default();
        let body = field_str(&doc, index.schema.body).unwrap_or_default();
        let path = field_str(&doc, index.schema.path).unwrap_or_default();
        let snippet = make_snippet(&body, &q_lower);
        hits.push(SearchHit {
            project_id,
            project_name,
            kind,
            title,
            snippet,
            path,
            score,
        });
    }
    Ok(hits)
}

fn field_str(doc: &TantivyDocument, field: Field) -> Option<String> {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn make_snippet(body: &str, q_lower: &str) -> String {
    if body.is_empty() {
        return String::new();
    }
    let lower = body.to_lowercase();
    let pos = lower.find(q_lower);
    let (start, end) = match pos {
        Some(p) => {
            let s = p.saturating_sub(40);
            let e = (p + q_lower.len() + 80).min(body.len());
            (s, e)
        }
        None => (0, SNIPPET_LEN.min(body.len())),
    };
    let mut s = start;
    while !body.is_char_boundary(s) && s < body.len() {
        s += 1;
    }
    let mut e = end;
    while !body.is_char_boundary(e) && e > s {
        e -= 1;
    }
    let mut out = String::new();
    if s > 0 {
        out.push_str("...");
    }
    out.push_str(&body[s..e]);
    if e < body.len() {
        out.push_str("...");
    }
    out
}

fn collect_md(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let p = entry.path();
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else { continue };
        if p.is_dir() {
            if name.starts_with('.') {
                continue;
            }
            if DEFAULT_EXCLUDE_DIRS.contains(&name) {
                continue;
            }
            collect_md(root, &p, out);
            continue;
        }
        if !p.is_file() {
            continue;
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if ext != "md" {
            continue;
        }
        let body = fs::read_to_string(&p).unwrap_or_default();
        let rel = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
        out.push((rel, body));
    }
}

fn title_from_path_or_frontmatter(body: &str, rel: &Path) -> String {
    // 한국어처럼 멀티바이트 문자가 4096 경계에 걸리면 직접 슬라이싱이 panic.
    // 가장 가까운 이전 char boundary 까지 자른다.
    let mut head_len = body.len().min(TITLE_SCAN_BYTES);
    while head_len > 0 && !body.is_char_boundary(head_len) {
        head_len -= 1;
    }
    let head = &body[..head_len];
    if let Some(fm) = extract_frontmatter(head) {
        if let Some(t) = fm
            .extra
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
        {
            return t;
        }
    }
    rel.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::new_project_entry;
    use tempfile::TempDir;

    fn project_in(dir: &Path, name: &str) -> ProjectEntry {
        let root = dir.join(name);
        fs::create_dir_all(root.join("docs")).unwrap();
        new_project_entry(root.clone(), root, Some(name.to_string()))
    }

    fn write_file(p: &Path, content: &str) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    #[test]
    fn open_or_create_creates_index_dir() {
        let dir = TempDir::new().unwrap();
        let idx_dir = dir.path().join("idx");
        let _ = open_or_create(&idx_dir).expect("open");
        assert!(idx_dir.is_dir());
    }

    #[test]
    fn open_or_create_errors_when_path_is_file() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("not-a-dir");
        fs::write(&p, "x").unwrap();
        let r = open_or_create(&p);
        assert!(r.is_err());
    }

    #[test]
    fn reindex_indexes_only_md_files_when_no_graphify() {
        let dir = TempDir::new().unwrap();
        let proj = project_in(dir.path(), "p1");
        write_file(&proj.docs_path.join("a.md"), "hello world");
        write_file(&proj.docs_path.join("ignore.txt"), "nope");
        write_file(&proj.docs_path.join("sub/b.md"), "deep content");

        let idx = open_or_create(&dir.path().join("idx")).unwrap();
        let n = reindex_project(&idx, &proj).unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn reindex_includes_graphify_nodes_when_present() {
        let dir = TempDir::new().unwrap();
        let proj = project_in(dir.path(), "p1");
        write_file(&proj.docs_path.join("a.md"), "alpha");
        let out = proj.graphify_out_path.clone();
        fs::create_dir_all(&out).unwrap();
        let json = r#"{"directed":false,"multigraph":false,"graph":{},"nodes":[{"id":"n1","label":"NodeOne"},{"id":"n2","label":"NodeTwo"}],"links":[]}"#;
        fs::write(out.join("graph.json"), json).unwrap();

        let idx = open_or_create(&dir.path().join("idx")).unwrap();
        let n = reindex_project(&idx, &proj).unwrap();
        assert_eq!(n, 3); // 1 file + 2 nodes
    }

    #[test]
    fn reindex_includes_qa_when_memory_present() {
        let dir = TempDir::new().unwrap();
        let proj = project_in(dir.path(), "p1");
        let memory = proj.graphify_out_path.join("memory");
        fs::create_dir_all(&memory).unwrap();
        write_file(&memory.join("query_001.md"), "Q: what is jwt?\nA: a token format");
        // 잘못된 이름은 제외
        write_file(&memory.join("notes.md"), "ignored");

        let idx = open_or_create(&dir.path().join("idx")).unwrap();
        let n = reindex_project(&idx, &proj).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn reindex_is_idempotent_for_same_project() {
        let dir = TempDir::new().unwrap();
        let proj = project_in(dir.path(), "p1");
        write_file(&proj.docs_path.join("a.md"), "alpha");
        let idx = open_or_create(&dir.path().join("idx")).unwrap();
        let n1 = reindex_project(&idx, &proj).unwrap();
        let n2 = reindex_project(&idx, &proj).unwrap();
        assert_eq!(n1, 1);
        assert_eq!(n2, 1);
        // 검색 결과도 1개여야 함 (중복 누적 X)
        let hits = search(&idx, "alpha", None, None, 10).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn reindex_other_project_does_not_remove_first() {
        let dir = TempDir::new().unwrap();
        let p1 = project_in(dir.path(), "p1");
        let p2 = project_in(dir.path(), "p2");
        write_file(&p1.docs_path.join("a.md"), "alpha keyword");
        write_file(&p2.docs_path.join("b.md"), "beta");
        let idx = open_or_create(&dir.path().join("idx")).unwrap();
        reindex_project(&idx, &p1).unwrap();
        reindex_project(&idx, &p2).unwrap();
        let hits = search(&idx, "alpha", None, None, 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].project_id, p1.id);
    }

    #[test]
    fn search_filters_by_project() {
        let dir = TempDir::new().unwrap();
        let p1 = project_in(dir.path(), "p1");
        let p2 = project_in(dir.path(), "p2");
        write_file(&p1.docs_path.join("a.md"), "common keyword");
        write_file(&p2.docs_path.join("b.md"), "common keyword");
        let idx = open_or_create(&dir.path().join("idx")).unwrap();
        reindex_project(&idx, &p1).unwrap();
        reindex_project(&idx, &p2).unwrap();
        let only_p2 = search(&idx, "common", Some(&[p2.id.clone()]), None, 10).unwrap();
        assert_eq!(only_p2.len(), 1);
        assert_eq!(only_p2[0].project_id, p2.id);
    }

    #[test]
    fn search_filters_by_kind() {
        let dir = TempDir::new().unwrap();
        let proj = project_in(dir.path(), "p1");
        write_file(&proj.docs_path.join("a.md"), "alpha");
        let out = proj.graphify_out_path.clone();
        fs::create_dir_all(&out).unwrap();
        let json = r#"{"directed":false,"multigraph":false,"graph":{},"nodes":[{"id":"n1","label":"alpha node"}],"links":[]}"#;
        fs::write(out.join("graph.json"), json).unwrap();

        let idx = open_or_create(&dir.path().join("idx")).unwrap();
        reindex_project(&idx, &proj).unwrap();

        let only_files = search(&idx, "alpha", None, Some(&[SearchKind::File]), 10).unwrap();
        assert_eq!(only_files.len(), 1);
        assert_eq!(only_files[0].kind, SearchKind::File);

        let only_nodes = search(&idx, "alpha", None, Some(&[SearchKind::Node]), 10).unwrap();
        assert_eq!(only_nodes.len(), 1);
        assert_eq!(only_nodes[0].kind, SearchKind::Node);
    }

    #[test]
    fn reindex_handles_korean_body_at_char_boundary() {
        // 4096바이트 경계에 한글이 걸리도록 본문을 구성. 회귀 테스트.
        let dir = TempDir::new().unwrap();
        let proj = project_in(dir.path(), "p1");
        // 한글 한 글자 = 3바이트. 4095/3 = 1365 → 1365자 후 다음 글자가 4095..4097 영역에 걸림.
        let mut body = String::with_capacity(8192);
        for _ in 0..2000 {
            body.push('한');
        }
        write_file(&proj.docs_path.join("ko.md"), &body);
        let idx = open_or_create(&dir.path().join("idx")).unwrap();
        let n = reindex_project(&idx, &proj).expect("must not panic on char boundary");
        assert_eq!(n, 1);
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let dir = TempDir::new().unwrap();
        let idx = open_or_create(&dir.path().join("idx")).unwrap();
        let hits = search(&idx, "  ", None, None, 10).unwrap();
        assert!(hits.is_empty());
    }
}
