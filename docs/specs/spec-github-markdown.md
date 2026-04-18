# Spec: GitHub 스타일 마크다운 렌더러

## 범위

Slice 1.3a의 Spec. 볼트 노트 뷰어를 GitHub 수준의 가독성으로 끌어올린다.
수식/다이어그램은 `spec-diagrams.md`에서 분리 정의한다.

- GFM 확장 (테이블, 각주, 체크박스, 취소선, 스마트 구두점)
- GitHub 시각 스타일 (`github-markdown-css`) 적용
- 코드 블록 하이라이팅 (Prism.js)
- 기존 위키링크/프런트매터 동작 회귀 없음

## 아키텍처

```
(Rust) render_note
  → pulldown-cmark (GFM 옵션 확장)
  → RenderedNote.html
         ↓
(Svelte) view/+page.svelte
  → <article class="markdown-body">{@html note.html}</article>
  → onMount: await highlightCode(article)   // Prism, lazy import
```

Rust는 "HTML 문자열 생산자", 프런트엔드는 "후처리 + 스타일링"을 담당한다.

## Public Interface

### Rust

```rust
// vault/renderer.rs — 시그니처 변경 없음
pub fn render_note(
    path: &Path,
    vault_root: &Path,
    backlinks: &[BacklinkEntry],
) -> Result<RenderedNote, AppError>

// 내부 헬퍼 — Options 확장만 수정
fn markdown_to_html(markdown: &str) -> String
```

### 프런트엔드

```ts
// src/lib/markdown/highlight.ts (신규)
/**
 * `.markdown-body` 내부의 코드 블록을 Prism으로 하이라이트한다.
 * language-mermaid / language-dbml 블록은 건너뛴다 (다이어그램 단계에서 처리).
 * Prism 코어/언어 번들은 lazy import한다.
 */
export async function highlightCode(root: HTMLElement): Promise<void>
```

## Behavior Contract — Rust GFM 옵션

| # | Given                                          | When          | Then                                                                                   |
| - | ---------------------------------------------- | ------------- | -------------------------------------------------------------------------------------- |
| 1 | 기존 14개 렌더러 테스트 (Slice 1.2 시점)       | `cargo test`  | 전부 통과 (회귀 없음)                                                                  |
| 2 | 파이프 테이블 마크다운                         | `render_note` | `<table>…<thead>…</thead><tbody>…</tbody></table>` 생성                                |
| 3 | `- [x] done` / `- [ ] todo` 체크박스           | `render_note` | `<input type="checkbox" disabled checked>` / `disabled` 포함                           |
| 4 | `~~취소선~~`                                    | `render_note` | `<del>취소선</del>`                                                                    |
| 5 | 각주 `[^1]` + 본문 하단 `[^1]: …`              | `render_note` | `<sup class="footnote-reference"><a …>`와 하단 `<div class="footnote-definition">` 생성 |
| 6 | 스마트 구두점 (`"quoted"`, `---`, `...`)        | `render_note` | 각각 `&ldquo;…&rdquo;`, `&mdash;`, `&hellip;`로 치환                                   |
| 7 | ```` ```rust\nfn …\n``` ````                   | `render_note` | `<pre><code class="language-rust">…</code></pre>` 생성                                 |
| 8 | ```` ```mermaid\nflowchart …\n``` ````         | `render_note` | `<pre><code class="language-mermaid">…</code></pre>` — 원문 코드 보존                  |
| 9 | ```` ```dbml\nTable users { … }\n``` ````      | `render_note` | `<pre><code class="language-dbml">…</code></pre>` — 원문 코드 보존                     |
| 10 | 인라인 `$E=mc^2$` / 블록 `$$\int x dx$$`      | `render_note` | `$` 구분자가 HTML 텍스트 노드에 **그대로** 남음 (프런트 KaTeX 후처리 위함)             |
| 11 | 코드 블록 안 위키링크 `` `[[foo]]` ``           | `render_note` | 위키링크로 변환되지 **않음** (기존 동작 유지)                                          |

## Behavior Contract — 프런트 `highlightCode`

| # | Given                                           | When                              | Then                                                                 |
| - | ----------------------------------------------- | --------------------------------- | -------------------------------------------------------------------- |
| A | `.markdown-body` 안에 `language-rust` 코드 블록 | `await highlightCode(root)`       | 해당 `<code>`에 Prism 토큰 `<span class="token …">` 삽입             |
| B | `language-mermaid` 블록                         | `await highlightCode(root)`       | 토큰 치환 없이 원문 유지 (다이어그램 단계에서 처리)                  |
| C | 언어 태그 없는 ` ``` ` 블록                     | `await highlightCode(root)`       | 변경 없음 (Prism plain 처리)                                         |
| D | 지원하지 않는 언어(예: `nim`)                   | `await highlightCode(root)`       | 콘솔 경고 1회 + 원문 유지, 이후 블록은 정상 처리                     |
| E | 같은 뷰로 다른 노트 재진입                      | `highlightCode` 연속 호출         | 이전 토큰이 누적되지 않음 (idempotent, 이미 하이라이트된 블록 스킵)  |

## Edge Cases

- **수식 토큰 보존**: pulldown-cmark는 `$`를 특수 처리하지 않지만, `$...$` 사이가 우연히 마크다운 인라인 문법과 충돌(`*`, `_` 등)할 수 있다. Spec 1.3a에서는 별도 이스케이프를 하지 않고, 사용자에게 백슬래시 또는 블록 `$$` 사용을 안내한다. KaTeX 단계에서 탐지/치환한다.
- **대용량 노트**: Prism은 동기 API지만 큰 블록에서 프레임을 막을 수 있다. `highlightCode`는 `requestIdleCallback`이 있으면 블록 단위로 나눠 처리, 없으면 즉시 실행.
- **서식 중첩 코드 블록**: GFM 펜스드(` ``` `) 안에 4-space 들여쓰기는 그대로 텍스트 보존.
- **XSS**: 입력 마크다운은 로컬 볼트 파일. pulldown-cmark는 raw HTML을 기본 허용 → 현재도 같은 신뢰 가정을 유지한다 (별도 sanitization은 본 Spec 밖).

## 렌더 순서 계약

프런트 파이프라인 전체 순서는 Slice 1.3b Spec에서 확정한다. 본 Spec은 다음 계약만 고정:

1. Rust는 `language-*` 클래스가 있는 `<pre><code>`를 생성한다.
2. `highlightCode`는 `language-mermaid`, `language-dbml`을 **건드리지 않는다**.
3. `highlightCode`는 KaTeX 단계 이후 실행된다 (KaTeX가 생성한 노드는 `<pre>` 바깥이라 충돌 없음).

## Dependencies

### Rust
- `pulldown-cmark 0.13` — 이미 사용 중. 추가 의존성 없음.

### 프런트엔드 (신규)
- `prismjs` (core + 언어 번들) — 코드 하이라이팅
- `github-markdown-css` — `.markdown-body` 시각 스타일

번들 전략:
- 두 패키지 모두 **동적 import**로 노트 뷰어 진입 시점에만 로드
- Prism 언어 번들 프리로드 셋:
  - 프로그래밍: `rust`, `kotlin`, `java`, `go`, `typescript`, `javascript`, `python`, `c`, `cpp`, `csharp`, `swift`, `ruby`
  - 스크립트/셸: `bash`, `sh`, `powershell`
  - 데이터/마크업: `json`, `yaml`, `toml`, `markdown`, `sql`, `html`, `css`, `diff`
  - 그 외 언어는 Prism `autoloader`로 on-demand 로드

### Mock boundary
- Rust 테스트: 파일 I/O 없는 단위 테스트 (`markdown_to_html` 직접 호출)
- 프런트 테스트(향후): jsdom 기반. 본 Slice 내에서는 샘플 노트로 수동 검증

## 디자인 토큰 통합

- `.markdown-body`에 GitHub CSS를 불러오되, 다음 토큰만 덮어쓴다:
  - `--color-canvas-default` → `--color-surface`
  - `--color-canvas-subtle` → `--color-surface-2`
  - `--color-fg-default` → `--color-text`
  - `--color-accent-fg` → `--color-accent`
  - `--color-border-default` → `--color-border`
- 본문 폰트 크기: `.markdown-body { font-size: var(--font-size-body); }`
  - 컴팩트 모드에서 자동으로 축소 (Slice 1.2의 `[data-density="compact"]`가 `--font-size-body`를 이미 축소)
- 코드 블록: Prism의 테마 CSS는 사용하지 않고, `.markdown-body pre` 스코프 안에서 토큰 기반으로 직접 색 정의.

## 제외 (Out of scope)

- 수식 렌더링 (KaTeX) — `spec-diagrams.md`
- Mermaid / DBML 렌더링 — `spec-diagrams.md`
- 마크다운 에디터/편집 — v1.0 범위 밖
- HTML sanitization 강화 — 기존 신뢰 모델 유지
- Shiki로의 하이라이터 교체 — MVP는 Prism 확정

## 테스트 계획

### Rust (TDD Red → Green)
- 신규 테스트: 각 Behavior Contract 항목(2–10)에 대해 최소 1개씩, 총 9개 추가
- 기존 테스트 14개 회귀 확인
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`

### 프런트엔드 (수동)
- `docs/samples/markdown-demo.md` 샘플 노트 작성 (Slice 1.3 공용)
- 뷰어에서 열어 다음 확인:
  - 테이블/체크박스/각주/취소선/스마트 구두점
  - rust/ts/python/bash 코드 하이라이팅
  - wikilink 클릭 동작 유지
  - 컴팩트 모드 토글 시 폰트/간격 축소
