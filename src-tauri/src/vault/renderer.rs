use std::fs;
use std::path::Path;

use pulldown_cmark::{html, Options, Parser};
use regex::Regex;

use crate::error::AppError;
use crate::models::{BacklinkEntry, RenderedNote};
use crate::vault::parser::{extract_frontmatter, extract_wikilinks, strip_frontmatter, title_from_path};

/// 위키링크를 HTML 앵커로 변환 (코드 블록 내부 제외)
fn convert_wikilinks(markdown: &str) -> String {
    let re = Regex::new(r"\[\[([^\]|]+)(?:\|([^\]]+))?\]\]").expect("invalid regex");
    let mut result = String::with_capacity(markdown.len());
    let mut in_code_block = false;


    for line in markdown.lines() {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_code_block {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // 인라인 코드 구간을 고려하여 변환
        let mut last_end = 0;
        let chars: Vec<char> = line.chars().collect();

        let mut segments: Vec<(usize, usize, bool)> = Vec::new(); // (start, end, is_code)

        // 인라인 코드 구간 찾기
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '`' {
                let code_start = i;
                i += 1;
                while i < chars.len() && chars[i] != '`' {
                    i += 1;
                }
                if i < chars.len() {
                    // 바이트 오프셋 계산
                    let byte_start: usize = chars[..code_start].iter().map(|c| c.len_utf8()).sum();
                    let byte_end: usize = chars[..=i].iter().map(|c| c.len_utf8()).sum();
                    if last_end < byte_start {
                        segments.push((last_end, byte_start, false));
                    }
                    segments.push((byte_start, byte_end, true));
                    last_end = byte_end;
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        let line_len = line.len();
        if last_end < line_len {
            segments.push((last_end, line_len, false));
        }
        if segments.is_empty() {
            segments.push((0, line_len, false));
        }

        for (start, end, is_code) in &segments {
            let segment = &line[*start..*end];
            if *is_code {
                result.push_str(segment);
            } else {
                let replaced = re.replace_all(segment, |caps: &regex::Captures| {
                    let target = caps[1].trim();
                    let display = caps
                        .get(2)
                        .map(|m| m.as_str().trim())
                        .unwrap_or(target);
                    format!(r#"<a href="{target}" class="wikilink">{display}</a>"#)
                });
                result.push_str(&replaced);
            }
        }
        result.push('\n');
    }

    // 마지막 줄바꿈 제거 (원본에 없었던 경우)
    if !markdown.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }
    result
}

/// 마크다운 → HTML 변환
fn markdown_to_html(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// 단일 노트를 HTML로 렌더링
pub fn render_note(
    path: &Path,
    vault_root: &Path,
    backlinks: &[BacklinkEntry],
) -> Result<RenderedNote, AppError> {
    let content = fs::read_to_string(path)?;

    let relative_path = path
        .strip_prefix(vault_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let frontmatter = extract_frontmatter(&content);
    let outgoing_links = extract_wikilinks(&content);
    let title = title_from_path(path);

    let body = strip_frontmatter(&content);
    let wikilinked = convert_wikilinks(body);
    let html = markdown_to_html(&wikilinked);

    Ok(RenderedNote {
        path: relative_path,
        title,
        frontmatter,
        html,
        outgoing_links,
        backlinks: backlinks.to_vec(),
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

    // --- BC #1: 일반 마크다운 → HTML, frontmatter 제외 ---
    #[test]
    fn test_render_basic_markdown() {
        let content = "---\ntype: til\ncreated: 2026-04-16\ntags: []\n---\n# Hello\n\nWorld";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let rendered = render_note(f.path(), vault_root, &[]).unwrap();
        assert!(rendered.html.contains("<h1>Hello</h1>"));
        assert!(rendered.html.contains("<p>World</p>"));
        assert!(!rendered.html.contains("type: til"));
    }

    // --- BC #2: 위키링크 → HTML 앵커 ---
    #[test]
    fn test_render_wikilink() {
        let content = "Check [[note-a]] here";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let rendered = render_note(f.path(), vault_root, &[]).unwrap();
        assert!(rendered.html.contains(r#"<a href="note-a" class="wikilink">note-a</a>"#));
    }

    // --- BC #3: 앨리어스 위키링크 ---
    #[test]
    fn test_render_wikilink_with_alias() {
        let content = "Check [[note-a|표시텍스트]] here";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let rendered = render_note(f.path(), vault_root, &[]).unwrap();
        assert!(rendered.html.contains(r#"<a href="note-a" class="wikilink">표시텍스트</a>"#));
    }

    // --- BC #4: frontmatter가 html에 미포함 ---
    #[test]
    fn test_render_excludes_frontmatter() {
        let content = "---\ntype: til\ncreated: 2026-04-16\ntags: [rust]\nstatus: seedling\n---\n# Title";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let rendered = render_note(f.path(), vault_root, &[]).unwrap();
        assert!(!rendered.html.contains("---"));
        assert!(!rendered.html.contains("seedling"));
        assert!(rendered.frontmatter.is_some());
    }

    // --- BC #5: 백링크 전달 ---
    #[test]
    fn test_render_passes_backlinks() {
        let content = "# Note";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let backlinks = vec![BacklinkEntry {
            path: "other.md".to_string(),
            title: "Other".to_string(),
            note_type: Some(NoteType::Til),
            context: "links to [[note]]".to_string(),
        }];

        let rendered = render_note(f.path(), vault_root, &backlinks).unwrap();
        assert_eq!(rendered.backlinks.len(), 1);
        assert_eq!(rendered.backlinks[0].path, "other.md");
    }

    // --- BC #6: 존재하지 않는 파일 → 에러 ---
    #[test]
    fn test_render_nonexistent_file() {
        let result = render_note(
            Path::new("/nonexistent/file.md"),
            Path::new("/nonexistent"),
            &[],
        );
        assert!(result.is_err());
    }

    // --- BC #7: frontmatter 없는 노트 ---
    #[test]
    fn test_render_no_frontmatter() {
        let content = "# Just markdown\n\nSome content";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let rendered = render_note(f.path(), vault_root, &[]).unwrap();
        assert!(rendered.frontmatter.is_none());
        assert!(rendered.html.contains("<h1>Just markdown</h1>"));
    }

    // --- BC #8: 빈 본문 (frontmatter만) ---
    #[test]
    fn test_render_empty_body() {
        let content = "---\ntype: til\ncreated: 2026-04-16\ntags: []\n---\n";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let rendered = render_note(f.path(), vault_root, &[]).unwrap();
        assert!(rendered.html.trim().is_empty());
    }

    // --- Edge: 코드 블록 안 위키링크는 변환하지 않음 ---
    #[test]
    fn test_wikilink_in_code_block_preserved() {
        let content = "Normal [[link]]\n\n```\n[[code-link]]\n```";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let rendered = render_note(f.path(), vault_root, &[]).unwrap();
        assert!(rendered.html.contains(r#"<a href="link" class="wikilink">link</a>"#));
        assert!(!rendered.html.contains(r#"<a href="code-link""#));
        assert!(rendered.html.contains("[[code-link]]"));
    }

    // --- Edge: 인라인 코드 안 위키링크도 보존 ---
    #[test]
    fn test_wikilink_in_inline_code_preserved() {
        let content = "Normal [[link]] and `[[inline-code]]`";
        let f = write_temp_md(content);
        let vault_root = f.path().parent().unwrap();

        let rendered = render_note(f.path(), vault_root, &[]).unwrap();
        assert!(rendered.html.contains(r#"<a href="link" class="wikilink">link</a>"#));
        assert!(!rendered.html.contains(r#"<a href="inline-code""#));
    }

    // --- 헬퍼 단위 테스트 ---
    #[test]
    fn test_strip_frontmatter_with_fm() {
        let content = "---\ntype: til\ncreated: 2026-04-16\n---\n# Hello";
        let body = strip_frontmatter(content);
        assert_eq!(body, "# Hello");
    }

    #[test]
    fn test_strip_frontmatter_without_fm() {
        let content = "# No frontmatter";
        let body = strip_frontmatter(content);
        assert_eq!(body, "# No frontmatter");
    }

    #[test]
    fn test_convert_wikilinks_basic() {
        let result = convert_wikilinks("Hello [[target]] world");
        assert!(result.contains(r#"<a href="target" class="wikilink">target</a>"#));
    }

    #[test]
    fn test_convert_wikilinks_alias() {
        let result = convert_wikilinks("Hello [[target|display]] world");
        assert!(result.contains(r#"<a href="target" class="wikilink">display</a>"#));
    }

    #[test]
    fn test_markdown_to_html_basic() {
        let html = markdown_to_html("# Title\n\nParagraph");
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<p>Paragraph</p>"));
    }
}
