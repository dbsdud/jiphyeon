# Spec: vault/renderer

## Public Interface

```rust
/// 단일 노트를 HTML로 렌더링
pub fn render_note(
    path: &Path,                   // 노트 절대 경로
    vault_root: &Path,             // 볼트 루트
    backlinks: &[BacklinkEntry],   // 인덱서에서 조회한 백링크
) -> Result<RenderedNote, AppError>
```

## 핵심 책임

1. 파일 읽기 → frontmatter 제거 → 본문 추출
2. 위키링크(`[[target]]`, `[[target|alias]]`) → HTML 앵커로 변환
3. pulldown-cmark으로 마크다운 → HTML 변환
4. `RenderedNote` 조립 (기존 파서 함수 재활용)

## Behavior Contract

| # | Given | When | Then |
|---|-------|------|------|
| 1 | 일반 마크다운 노트 | render_note | HTML로 변환, frontmatter 제외 |
| 2 | 위키링크 `[[note-a]]` 포함 | render_note | `<a href="note-a" class="wikilink">note-a</a>`로 변환 |
| 3 | 앨리어스 위키링크 `[[note-a\|표시]]` | render_note | `<a href="note-a" class="wikilink">표시</a>`로 변환 |
| 4 | frontmatter 있는 노트 | render_note | html에 frontmatter YAML 미포함 |
| 5 | 백링크 제공됨 | render_note | RenderedNote.backlinks에 그대로 전달 |
| 6 | 존재하지 않는 파일 | render_note | AppError 반환 |
| 7 | frontmatter 없는 노트 | render_note | frontmatter: None, 본문 전체 렌더링 |
| 8 | 빈 본문 (frontmatter만) | render_note | html: 빈 문자열 |

## Edge Cases

- 위키링크가 코드 블록 안에 있을 때 → 변환하지 않음 (코드 블록 보존)
- 중첩/깨진 위키링크 `[[[bad]]` → 그대로 출력

## Dependencies

- `parser::extract_frontmatter` — frontmatter 파싱 재활용
- `parser::extract_wikilinks` — outgoing_links 추출 재활용
- `pulldown-cmark` — 마크다운 → HTML
- Mock boundary: 없음 (순수 함수 + 파일 I/O만)

## 내부 헬퍼

```rust
/// frontmatter 블록 제거 후 본문 반환
fn strip_frontmatter(content: &str) -> &str

/// 위키링크를 HTML 앵커로 변환 (코드 블록 내부 제외)
fn convert_wikilinks(markdown: &str) -> String

/// 마크다운 → HTML 변환
fn markdown_to_html(markdown: &str) -> String
```
