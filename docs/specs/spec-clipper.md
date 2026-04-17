# Spec: clipper

## Public Interface

```rust
/// URL → 마크다운 클리핑 파이프라인
pub async fn clip_url(request: &ClipRequest, vault_path: &Path) -> Result<ClipResult, AppError>
```

## 내부 헬퍼

```rust
/// HTTP GET으로 HTML 가져오기
async fn fetch_html(url: &str) -> Result<String, AppError>

/// HTML에서 본문/제목/저자 추출
fn extract_article(html: &str) -> ExtractedContent

/// HTML → Markdown 변환
fn html_to_markdown(html: &str) -> String

/// 제목 → URL-safe slug
fn slugify(title: &str) -> String
```

## 새 타입

```rust
struct ExtractedContent {
    title: String,
    author: Option<String>,
    content_html: String,
}
```

## Behavior Contract

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 유효한 HTML (title + article) | extract_article | title, content_html 추출 |
| 2 | og:title 메타 태그 | extract_article | og:title을 title로 사용 |
| 3 | author 메타 태그 | extract_article | author 추출 |
| 4 | article 태그 없는 HTML | extract_article | body에서 nav/header/footer 제거 후 추출 |
| 5 | HTML 본문 | html_to_markdown | 마크다운으로 변환 |
| 6 | 한글/특수문자 제목 | slugify | ASCII slug 생성 |
| 7 | 전체 파이프라인 | clip_url | inbox/{date}-{slug}.md 저장, frontmatter 포함 |
| 8 | 존재하지 않는 URL | fetch_html | AppError 반환 |

## Edge Cases

- 빈 본문 → 빈 마크다운 저장 (에러 아님)
- inbox/ 디렉토리 미존재 → 자동 생성
- title 없는 HTML → URL에서 제목 추출

## Dependencies

- `reqwest` — HTTP GET (async)
- `scraper` — HTML 파싱/본문 추출
- `html2md` — HTML → Markdown
- Mock boundary: `fetch_html` (네트워크 I/O)

## Frontmatter 템플릿

```yaml
---
type: clipping
created: {YYYY-MM-DD}
source: {url}
author: {extracted or ""}
status: seedling
tags: {request.tags or []}
---
```

## 파일 저장

- 경로: `{vault_path}/inbox/{YYYY-MM-DD}-{slug}.md`
- inbox/ 없으면 자동 생성
