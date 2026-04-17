use std::fs;
use std::path::Path;

use regex::Regex;

use crate::error::AppError;
use crate::models::{Frontmatter, NoteEntry};

/// 파일명에서 확장자를 제거한 제목 추출
pub fn title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("untitled")
        .to_string()
}

/// 마크다운 본문에서 YAML frontmatter 추출
pub fn extract_frontmatter(content: &str) -> Option<Frontmatter> {
    if !content.starts_with("---") {
        return None;
    }

    let rest = &content[3..];
    let end = rest.find("\n---")?;
    let yaml_str = &rest[..end].trim();

    serde_yaml_ng::from_str(yaml_str).ok()
}

/// frontmatter를 제거한 본문 텍스트 추출
pub fn strip_frontmatter(content: &str) -> &str {
    if !content.starts_with("---") {
        return content;
    }
    let rest = &content[3..];
    match rest.find("\n---") {
        Some(end) => {
            let after = &rest[end + 4..];
            after.strip_prefix('\n').unwrap_or(after)
        }
        None => content,
    }
}

/// 마크다운 본문에서 [[wikilink]] 대상 추출
pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]").expect("invalid regex");
    re.captures_iter(content)
        .map(|cap| cap[1].trim().to_string())
        .collect()
}

/// .md 파일을 파싱하여 NoteEntry 생성
pub fn parse_note(path: &Path, vault_root: &Path) -> Result<NoteEntry, AppError> {
    let content = fs::read_to_string(path)?;
    let metadata = fs::metadata(path)?;

    let relative_path = path
        .strip_prefix(vault_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let frontmatter = extract_frontmatter(&content);
    let outgoing_links = extract_wikilinks(&content);
    let title = title_from_path(path);
    let body = strip_frontmatter(&content).to_string();

    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    Ok(NoteEntry {
        path: relative_path,
        title,
        frontmatter,
        outgoing_links,
        modified_at,
        size: metadata.len(),
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::NoteType;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp_md(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::with_suffix(".md").unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn test_extract_frontmatter_valid() {
        let content = "---\ntype: til\ncreated: 2026-04-16\ntags:\n  - rust\nstatus: seedling\n---\n# Hello";
        let fm = extract_frontmatter(content).expect("should parse");
        assert_eq!(fm.note_type, NoteType::Til);
        assert_eq!(fm.tags, vec!["rust".to_string()]);
    }

    #[test]
    fn test_extract_frontmatter_none_when_missing() {
        let content = "# Just a heading\nSome content";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn test_extract_frontmatter_none_on_invalid_yaml() {
        let content = "---\ntype: [invalid\n---\n# Hello";
        assert!(extract_frontmatter(content).is_none());
    }

    #[test]
    fn test_extract_wikilinks() {
        let content = "Some text [[note-a]] and [[note-b|alias]] here";
        let links = extract_wikilinks(content);
        assert_eq!(links, vec!["note-a", "note-b"]);
    }

    #[test]
    fn test_extract_wikilinks_empty() {
        let content = "No links here";
        let links = extract_wikilinks(content);
        assert!(links.is_empty());
    }

    #[test]
    fn test_title_from_path() {
        let path = Path::new("dev/2026-04-16-my-til.md");
        assert_eq!(title_from_path(path), "2026-04-16-my-til");
    }

    #[test]
    fn test_parse_note_with_frontmatter() {
        let content = "---\ntype: til\ncreated: 2026-04-16\ntags: []\n---\n# Hello\n[[other-note]]";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let entry = parse_note(f.path(), vault_root).expect("should parse");
        assert!(entry.frontmatter.is_some());
        assert_eq!(entry.frontmatter.as_ref().unwrap().note_type, NoteType::Til);
        assert_eq!(entry.outgoing_links, vec!["other-note"]);
    }

    #[test]
    fn test_parse_note_without_frontmatter() {
        let content = "# Just markdown\nSome content";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let entry = parse_note(f.path(), vault_root).expect("should parse");
        assert!(entry.frontmatter.is_none());
        assert!(entry.outgoing_links.is_empty());
    }

    #[test]
    fn test_parse_note_invalid_yaml_graceful() {
        let content = "---\ntype: [broken\n---\n# Hello";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let entry = parse_note(f.path(), vault_root).expect("should not fail");
        assert!(entry.frontmatter.is_none());
    }
}
