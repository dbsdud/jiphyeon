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
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_SMART_PUNCTUATION;
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

    // ─────────────────────────────────────────────────────
    // Slice 1.3a — spec-github-markdown.md Behavior Contract
    // ─────────────────────────────────────────────────────

    // BC #2: 파이프 테이블
    #[test]
    fn test_gfm_table() {
        let html = markdown_to_html("| A | B |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<table>"));
        assert!(html.contains("<thead>"));
        assert!(html.contains("<tbody>"));
        assert!(html.contains("<th>A</th>"));
        assert!(html.contains("<td>1</td>"));
    }

    // BC #3: 체크박스
    #[test]
    fn test_gfm_task_list() {
        let html = markdown_to_html("- [x] done\n- [ ] todo\n");
        assert!(html.contains(r#"<input disabled="" type="checkbox" checked=""/>"#));
        assert!(html.contains(r#"<input disabled="" type="checkbox"/>"#));
    }

    // BC #4: 취소선
    #[test]
    fn test_gfm_strikethrough() {
        let html = markdown_to_html("~~취소선~~");
        assert!(html.contains("<del>취소선</del>"));
    }

    // BC #5: 각주 (ENABLE_FOOTNOTES 필요)
    #[test]
    fn test_gfm_footnote() {
        let html = markdown_to_html("본문[^1]\n\n[^1]: 각주 내용\n");
        assert!(
            html.contains("footnote-reference") || html.contains("footnote"),
            "각주 참조 태그 없음: {html}"
        );
        assert!(
            html.contains("각주 내용"),
            "각주 정의 텍스트 없음: {html}"
        );
    }

    // BC #6: 스마트 구두점 (ENABLE_SMART_PUNCTUATION 필요)
    #[test]
    fn test_gfm_smart_punctuation() {
        let html = markdown_to_html(r#""quoted" -- dash ..."#);
        // pulldown-cmark smart punctuation: ", ", –, …
        assert!(
            html.contains('\u{201c}') || html.contains("&ldquo;"),
            "좌 큰따옴표 미변환: {html}"
        );
        assert!(
            html.contains('\u{201d}') || html.contains("&rdquo;"),
            "우 큰따옴표 미변환: {html}"
        );
        assert!(
            html.contains('\u{2013}') || html.contains('\u{2014}'),
            "대시 미변환: {html}"
        );
        assert!(
            html.contains('\u{2026}') || html.contains("&hellip;"),
            "말줄임 미변환: {html}"
        );
    }

    // BC #7: 코드 블록은 language-* 클래스
    #[test]
    fn test_code_block_language_class() {
        let html = markdown_to_html("```rust\nfn main() {}\n```");
        assert!(
            html.contains(r#"<code class="language-rust">"#),
            "language-rust 클래스 없음: {html}"
        );
    }

    // BC #8: mermaid 블록은 language-mermaid 클래스로 보존
    #[test]
    fn test_mermaid_block_preserved() {
        let html = markdown_to_html("```mermaid\nflowchart TD\n  A --> B\n```");
        assert!(
            html.contains(r#"<code class="language-mermaid">"#),
            "language-mermaid 보존 실패: {html}"
        );
        assert!(html.contains("flowchart TD"));
        assert!(html.contains("A --&gt; B") || html.contains("A --> B"));
    }

    // BC #9: dbml 블록은 language-dbml 클래스로 보존
    #[test]
    fn test_dbml_block_preserved() {
        let html = markdown_to_html("```dbml\nTable users {\n  id int [pk]\n}\n```");
        assert!(
            html.contains(r#"<code class="language-dbml">"#),
            "language-dbml 보존 실패: {html}"
        );
        assert!(html.contains("Table users"));
    }

    // BC #10: 수식 구문은 HTML 텍스트에 그대로 통과 (프런트 KaTeX 후처리)
    #[test]
    fn test_math_syntax_passthrough() {
        let inline = markdown_to_html("인라인 $E=mc^2$ 끝");
        assert!(inline.contains("$E=mc^2$"), "인라인 수식 통과 실패: {inline}");

        let block = markdown_to_html("$$\n\\int_0^1 x^2 dx\n$$");
        assert!(block.contains("$$"), "블록 수식 구분자 통과 실패: {block}");
        assert!(
            block.contains("\\int_0^1 x^2 dx") || block.contains("int_0^1 x^2 dx"),
            "블록 수식 본문 통과 실패: {block}"
        );
    }
}
