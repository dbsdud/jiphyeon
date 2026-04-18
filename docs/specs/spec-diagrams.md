# Spec: 수식/다이어그램 렌더링 (KaTeX · Mermaid · DBML)

## 범위

Slice 1.3b의 Spec. 노트 뷰어 후처리 파이프라인에 수식/다이어그램 렌더러를 얹는다.

포함:
- `$...$`, `$$...$$` 수식 → KaTeX
- ```` ```mermaid ```` → Mermaid SVG
- ```` ```dbml ```` → DBML SVG (`@softwaretechnik/dbml-renderer`)

포함되지 않음:
- GFM/코드 하이라이팅 — `spec-github-markdown.md`
- PlantUML — Epic 1 범위 밖 (Java JRE 번들 부담으로 제외 확정)
- 수식 에디터/미리보기 — v1.0 범위 밖

## 아키텍처

```
view/+page.svelte — onMount(note 변경):
  article = <article class="markdown-body">{@html note.html}</article>
  await renderMarkdownPipeline(article, note.html)

renderMarkdownPipeline:
  1. renderMath(article)          // KaTeX — $...$ / $$...$$ 치환
  2. highlightCode(article)       // Prism (Slice 1.3a)
  3. renderMermaid(article)       // language-mermaid → SVG
  4. renderDbml(article)          // language-dbml  → SVG
```

**순서가 중요한 이유**:
1. KaTeX가 먼저 `$`를 소비해야 이후 단계에서 `$` 포함 코드가 안전하다.
2. Prism은 `language-mermaid`/`language-dbml`을 건너뛰므로(1.3a 계약), Mermaid/DBML이 원본을 읽을 수 있다.
3. Mermaid/DBML은 `<pre><code>`를 **`<div class="diagram">`으로 치환**하기 때문에, 토큰 하이라이팅이 끝난 뒤 실행해야 한다.

각 단계는 독립적으로 try/catch. 한 단계 실패가 다음 단계를 막지 않는다.

## Public Interface

```ts
// src/lib/markdown/pipeline.ts (신규)
export async function renderMarkdownPipeline(
  root: HTMLElement,
): Promise<PipelineReport>

export interface PipelineReport {
  math: StageReport
  highlight: StageReport
  mermaid: StageReport
  dbml: StageReport
}

export interface StageReport {
  succeeded: number      // 성공한 블록 수
  failed: number         // 실패한 블록 수 (원문 보존 + 오버레이)
  skipped: boolean       // 해당 문법이 노트에 없어 라이브러리 로드 스킵
}
```

```ts
// src/lib/markdown/math.ts
/** $...$ 인라인 + $$...$$ 블록 수식을 KaTeX로 치환한다. lazy import. */
export async function renderMath(root: HTMLElement): Promise<StageReport>

// src/lib/markdown/mermaid.ts
/** language-mermaid 코드 블록을 SVG로 치환한다. lazy import. */
export async function renderMermaid(root: HTMLElement): Promise<StageReport>

// src/lib/markdown/dbml.ts
/** language-dbml 코드 블록을 SVG로 치환한다. lazy import. */
export async function renderDbml(root: HTMLElement): Promise<StageReport>
```

## Behavior Contract — 파이프라인 전체

| # | Given                                                      | When                        | Then                                                                            |
| - | ---------------------------------------------------------- | --------------------------- | ------------------------------------------------------------------------------- |
| 1 | 수식 · 코드 · mermaid · dbml이 모두 있는 노트              | `renderMarkdownPipeline`    | 4단계 모두 실행, 최종 DOM에 KaTeX 수식 · Prism 토큰 · SVG 다이어그램 공존      |
| 2 | 수식 없는 노트                                             | `renderMarkdownPipeline`    | `math.skipped === true` (KaTeX 번들 로드하지 않음)                              |
| 3 | mermaid 블록 없는 노트                                     | `renderMarkdownPipeline`    | `mermaid.skipped === true` (Mermaid 번들 로드하지 않음)                         |
| 4 | 같은 노트를 재렌더                                         | `renderMarkdownPipeline` ×2 | 중복 치환 없음 (이미 처리된 노드는 건너뜀)                                      |
| 5 | Mermaid 파싱 실패가 있는 노트                              | `renderMarkdownPipeline`    | `mermaid.failed ≥ 1`, 이후 `renderDbml`은 정상 실행, 파이프라인 전체 성공 반환  |

## Behavior Contract — `renderMath`

| #  | Given                               | When          | Then                                                                 |
| -- | ----------------------------------- | ------------- | -------------------------------------------------------------------- |
| M1 | 인라인 `$E=mc^2$`                   | `renderMath`  | 위치에 `<span class="katex">…</span>` 삽입                           |
| M2 | 블록 `$$\int_0^1 x^2 dx$$`          | `renderMath`  | `<div class="katex-display">…</div>` 삽입                            |
| M3 | 코드 블록 안 `$foo$`                | `renderMath`  | 변환되지 않음 (`<pre>`, `<code>` 자손은 무시)                         |
| M4 | KaTeX 파싱 실패 `$\bad{x}$`         | `renderMath`  | 원본 `$\bad{x}$` 유지 + `<span class="katex-error">` 오버레이 + 콘솔 경고 |
| M5 | `$$` 블록이 열렸는데 닫히지 않음    | `renderMath`  | 변환하지 않음, 원문 유지                                              |
| M6 | 위키링크 `[[note$]]` 안의 `$`       | `renderMath`  | 링크 속성(`href`)은 건드리지 않음, 텍스트 노드만 순회                 |

## Behavior Contract — `renderMermaid`

| #   | Given                                        | When            | Then                                                                     |
| --- | -------------------------------------------- | --------------- | ------------------------------------------------------------------------ |
| MM1 | `language-mermaid` 블록 1개                  | `renderMermaid` | `<pre>`가 `<div class="diagram diagram-mermaid"><svg …></svg></div>`로 치환 |
| MM2 | 같은 노트 mermaid 블록 3개                   | `renderMermaid` | 모두 치환, 각각 고유 SVG ID                                              |
| MM3 | Mermaid 파싱 실패 블록                       | `renderMermaid` | 원본 `<pre>` 유지 + `<div class="diagram-error">Failed to render</div>` 오버레이 |
| MM4 | 컴팩트 모드                                  | `renderMermaid` | SVG는 `width: 100%`로 반응형, 컨테이너 폰트 축소 영향 받음               |
| MM5 | `mermaid` 번들 이미 로드됨 (다른 노트 앞서)  | `renderMermaid` | 동적 import 재호출되지 않음 (모듈 캐시 히트)                             |

## Behavior Contract — `renderDbml`

| #  | Given                        | When         | Then                                                                             |
| -- | ---------------------------- | ------------ | -------------------------------------------------------------------------------- |
| D1 | `language-dbml` 블록         | `renderDbml` | `<pre>`가 `<div class="diagram diagram-dbml"><svg …></svg></div>`로 치환         |
| D2 | DBML 문법 오류 블록          | `renderDbml` | 원본 `<pre>` 유지 + `.diagram-error` 오버레이 + 파서 메시지 `title` 속성 노출    |
| D3 | 빈 DBML 블록                 | `renderDbml` | 원본 유지, 오버레이 없음 (정책: 빈 입력은 에러 아님)                             |

## 라이브러리 로딩 전략

라이브러리는 **노트에 해당 문법이 있을 때만** 동적 import.

```ts
async function renderMath(root) {
  if (!hasMathSyntax(root)) return { succeeded: 0, failed: 0, skipped: true }
  const { default: katex } = await import("katex")
  await import("katex/dist/katex.min.css")   // side-effect
  // …
}
```

- `hasMathSyntax`: 본문 텍스트에 `/\$[^$\n]+\$|\$\$[\s\S]+?\$\$/` 매치 여부
- `hasMermaidBlock`: `root.querySelector("code.language-mermaid")` 존재 여부
- `hasDbmlBlock`: `root.querySelector("code.language-dbml")` 존재 여부

**번들 영향**:
- KaTeX ~300KB (CSS 포함 ~400KB)
- Mermaid ~500KB
- DBML ~400KB
- 초기 뷰어 진입 시 어느 것도 로드되지 않음 → 기존 체감 성능 유지

## 멱등성 / 재렌더

사용자가 다른 노트로 이동하면 `view/+page.svelte`는 `article` 컨테이너를 `{@html}`로 **완전 교체**한다.
따라서 각 단계는 "새 DOM 트리"를 전제로 하면 된다.

방어 차원에서 각 단계는 마커 속성을 활용:
- `data-katex-rendered`
- `data-mermaid-rendered`
- `data-dbml-rendered`

이미 마커가 있는 노드는 스킵 (동일 트리에서 파이프라인이 두 번 호출되는 에지 케이스 대비).

## 오류 처리

모든 단계는 **try/catch**로 감싸고, 실패해도 `StageReport`를 반환한다. 오류는:
1. 원본 노드 유지
2. 오버레이 DOM 추가 (`.diagram-error` 또는 `.katex-error`)
3. `console.warn(slice, error)` 한 번만 출력 (블록당)

파이프라인 전체가 죽지 않는다. 한 노트의 Mermaid가 깨져도 DBML/KaTeX는 정상 렌더된다.

## Dependencies (신규)

```json
{
  "katex": "^0.16",
  "mermaid": "^11",
  "@softwaretechnik/dbml-renderer": "^1"
}
```

Slice 1.3a의 `prismjs`, `github-markdown-css`와 함께 설치.

## Mock boundary

- 단위 테스트(본 Slice 범위 외 — 프런트 테스트 러너 미도입):
  - `hasMathSyntax`, `hasMermaidBlock`, `hasDbmlBlock`는 순수 함수라 jsdom 없이 테스트 가능
- 통합 검증: `docs/samples/markdown-demo.md` 샘플 노트 수동 확인

## 디자인 토큰 연계

```css
.markdown-body .diagram {
  background: var(--color-surface-2);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: calc(var(--spacing) * 3);
  overflow-x: auto;
}
.markdown-body .diagram-error {
  color: var(--color-danger);
  font-family: var(--font-mono);
  font-size: var(--font-size-small);
}
.markdown-body .katex-display {
  margin: calc(var(--spacing) * 4) 0;
  overflow-x: auto;
}
```

- **Mermaid 테마 런타임 분기**: `html[data-theme]` 속성을 읽어 초기화.
  - `data-theme="light"` (또는 미지정) → `default`
  - `data-theme="dark"` → `dark`
  - 테마 변경 이벤트 발생 시 `mermaid.initialize({ theme })` 재호출 + 뷰어 내 기존 SVG는 다음 렌더 때 교체 (즉시 재렌더는 1.3 범위 밖)
- 컴팩트 모드: 별도 오버라이드 없이 `--spacing` 축소로 자연스럽게 반영.

## 수용 기준 (검증)

`docs/samples/markdown-demo.md` 샘플 노트를 열었을 때:
- 인라인 수식 + 블록 수식 모두 정상 출력
- `flowchart TD`, `sequenceDiagram`, `erDiagram` 각 1개 이상 정상 렌더
- `Table users { id int [pk] }` DBML 1개 이상 정상 렌더
- 의도적 파싱 실패 블록 1개 → 원문 보존 + 오버레이 문구 표시
- 컴팩트 모드 전환 시 깨지지 않음
- `cargo test`/`npm run tauri dev` 둘 다 성공
