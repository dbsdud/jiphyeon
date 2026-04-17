use scraper::{Html, Selector};

/// HTML에서 추출된 콘텐츠
pub struct ExtractedContent {
    pub title: String,
    pub author: Option<String>,
    pub content_html: String,
}

/// HTML에서 본문/제목/저자 추출
pub fn extract_article(html: &str) -> ExtractedContent {
    let document = Html::parse_document(html);

    // 제목: og:title > title 태그
    let title = extract_og_title(&document)
        .or_else(|| extract_title_tag(&document))
        .unwrap_or_default();

    // 저자: meta[name=author] 또는 meta[property=article:author]
    let author = extract_meta(&document, "author")
        .or_else(|| extract_meta_property(&document, "article:author"));

    // 본문: article > main > body (nav/header/footer 제거)
    let content_html = extract_content(&document);

    ExtractedContent {
        title,
        author,
        content_html,
    }
}

fn extract_og_title(doc: &Html) -> Option<String> {
    let sel = Selector::parse(r#"meta[property="og:title"]"#).ok()?;
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_title_tag(doc: &Html) -> Option<String> {
    let sel = Selector::parse("title").ok()?;
    doc.select(&sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_meta(doc: &Html, name: &str) -> Option<String> {
    let sel = Selector::parse(&format!(r#"meta[name="{}"]"#, name)).ok()?;
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_meta_property(doc: &Html, property: &str) -> Option<String> {
    let sel = Selector::parse(&format!(r#"meta[property="{}"]"#, property)).ok()?;
    doc.select(&sel)
        .next()
        .and_then(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_content(doc: &Html) -> String {
    // article 태그 우선
    if let Some(html) = extract_by_selector(doc, "article") {
        return html;
    }
    // main 태그
    if let Some(html) = extract_by_selector(doc, "main") {
        return html;
    }
    // body에서 nav/header/footer/aside 제거
    extract_body_cleaned(doc)
}

fn extract_by_selector(doc: &Html, selector: &str) -> Option<String> {
    let sel = Selector::parse(selector).ok()?;
    doc.select(&sel).next().map(|el| el.inner_html())
}

fn extract_body_cleaned(doc: &Html) -> String {
    let body_sel = Selector::parse("body").unwrap();
    let body = match doc.select(&body_sel).next() {
        Some(b) => b,
        None => return String::new(),
    };

    let remove_tags = ["nav", "header", "footer", "aside", "script", "style"];
    let mut html = body.inner_html();

    for tag in &remove_tags {
        let sel = Selector::parse(tag).unwrap();
        for el in doc.select(&sel) {
            let outer = el.html();
            html = html.replace(&outer, "");
        }
    }

    html
}
