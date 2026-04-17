pub mod extractor;
pub mod converter;

use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::error::AppError;
use crate::models::{ClipRequest, ClipResult};

/// HTTP GET으로 HTML 가져오기
pub fn fetch_html(url: &str) -> Result<String, AppError> {
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Network(e.to_string()))?
        .get(url)
        .header("User-Agent", "Jiphyeon-Clipper/0.2")
        .send()
        .map_err(|e| AppError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "HTTP {}",
            response.status()
        )));
    }

    response.text().map_err(|e| AppError::Network(e.to_string()))
}

/// 제목 → URL-safe slug
pub fn slugify(title: &str) -> String {
    let s = slug::slugify(title);
    if s.is_empty() {
        "untitled".to_string()
    } else {
        s
    }
}

/// URL → 마크다운 클리핑 파이프라인
pub fn clip_url(request: &ClipRequest, vault_path: &Path) -> Result<ClipResult, AppError> {
    let html = fetch_html(&request.url)?;
    clip_url_with_html(request, vault_path, &html)
}

/// HTML을 직접 받아 클리핑 (테스트 + 내부 파이프라인)
pub fn clip_url_with_html(
    request: &ClipRequest,
    vault_path: &Path,
    html: &str,
) -> Result<ClipResult, AppError> {
    let extracted = extractor::extract_article(html);
    let markdown = converter::html_to_markdown(&extracted.content_html);
    let title = if extracted.title.is_empty() {
        "Untitled".to_string()
    } else {
        extracted.title
    };

    let today = Utc::now().format("%Y-%m-%d").to_string();
    let slug = slugify(&title);
    let filename = format!("{}-{}.md", today, slug);

    let inbox_dir = vault_path.join("inbox");
    fs::create_dir_all(&inbox_dir)?;

    let tags_str = request
        .tags
        .as_ref()
        .map(|tags| {
            let items: Vec<String> = tags.iter().map(|t| format!("  - {}", t)).collect();
            if items.is_empty() {
                "tags: []".to_string()
            } else {
                format!("tags:\n{}", items.join("\n"))
            }
        })
        .unwrap_or_else(|| "tags: []".to_string());

    let author_str = extracted
        .author
        .as_deref()
        .unwrap_or("");

    let content = format!(
        "---\ntype: clipping\ncreated: {}\nsource: {}\nauthor: \"{}\"\nstatus: seedling\n{}\n---\n\n{}",
        today, request.url, author_str, tags_str, markdown
    );

    let rel_path = format!("inbox/{}", filename);
    let full_path = vault_path.join(&rel_path);
    fs::write(&full_path, &content)?;

    Ok(ClipResult {
        path: rel_path,
        title,
        success: true,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- slugify ---

    #[test]
    fn test_slugify_ascii() {
        let result = slugify("Hello World");
        assert_eq!(result, "hello-world");
    }

    #[test]
    fn test_slugify_special_chars() {
        let result = slugify("Rust: A Systems Language!");
        assert!(!result.contains(':'));
        assert!(!result.contains('!'));
        assert!(!result.is_empty());
    }

    #[test]
    fn test_slugify_korean() {
        let result = slugify("한글 제목 테스트");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_slugify_empty() {
        let result = slugify("");
        assert_eq!(result, "untitled");
    }

    // --- extract_article ---

    #[test]
    fn test_extract_article_with_title_and_article() {
        let html = r#"
        <html>
        <head><title>Test Page</title></head>
        <body>
            <nav>nav</nav>
            <article><p>Article content</p></article>
            <footer>footer</footer>
        </body>
        </html>"#;

        let result = extractor::extract_article(html);
        assert_eq!(result.title, "Test Page");
        assert!(result.content_html.contains("Article content"));
        assert!(!result.content_html.contains("nav"));
    }

    #[test]
    fn test_extract_article_og_title() {
        let html = r#"
        <html>
        <head>
            <title>Fallback</title>
            <meta property="og:title" content="OG Title" />
        </head>
        <body><article><p>Content</p></article></body>
        </html>"#;

        let result = extractor::extract_article(html);
        assert_eq!(result.title, "OG Title");
    }

    #[test]
    fn test_extract_article_author_meta() {
        let html = r#"
        <html>
        <head>
            <title>Test</title>
            <meta name="author" content="John Doe" />
        </head>
        <body><article><p>Content</p></article></body>
        </html>"#;

        let result = extractor::extract_article(html);
        assert_eq!(result.author, Some("John Doe".to_string()));
    }

    #[test]
    fn test_extract_article_no_article_tag() {
        let html = r#"
        <html>
        <head><title>Test</title></head>
        <body>
            <nav>navigation</nav>
            <div><p>Main content here</p></div>
            <footer>footer</footer>
        </body>
        </html>"#;

        let result = extractor::extract_article(html);
        assert!(result.content_html.contains("Main content"));
    }

    // --- html_to_markdown ---

    #[test]
    fn test_html_to_markdown() {
        let html = "<h1>Title</h1><p>Paragraph with <strong>bold</strong></p>";
        let md = converter::html_to_markdown(html);
        assert!(md.contains("Title"), "should contain title, got: {}", md);
        assert!(md.contains("**bold**"), "should contain bold, got: {}", md);
    }

    // --- clip_url (통합, mock HTML) ---

    #[test]
    fn test_clip_url_creates_file() {
        let dir = TempDir::new().unwrap();
        let vault_path = dir.path();

        let html = r#"
        <html>
        <head><title>Test Clip</title></head>
        <body><article><p>Clipped content</p></article></body>
        </html>"#;

        let request = ClipRequest {
            url: "https://example.com/article".to_string(),
            tags: Some(vec!["test".to_string()]),
        };

        let result = clip_url_with_html(&request, vault_path, html).unwrap();
        assert!(result.success);
        assert!(result.path.starts_with("inbox/"));
        assert!(result.path.ends_with(".md"));

        let full_path = vault_path.join(&result.path);
        assert!(full_path.exists());

        let content = fs::read_to_string(full_path).unwrap();
        assert!(content.contains("type: clipping"));
        assert!(content.contains("source: https://example.com/article"));
        assert!(content.contains("status: seedling"));
        assert!(content.contains("Clipped content"));
    }

    #[test]
    fn test_clip_url_creates_inbox_dir() {
        let dir = TempDir::new().unwrap();
        let vault_path = dir.path();
        assert!(!vault_path.join("inbox").exists());

        let html = "<html><head><title>T</title></head><body><p>C</p></body></html>";
        let request = ClipRequest {
            url: "https://example.com".to_string(),
            tags: None,
        };

        let result = clip_url_with_html(&request, vault_path, html).unwrap();
        assert!(result.success);
        assert!(vault_path.join("inbox").exists());
    }
}
