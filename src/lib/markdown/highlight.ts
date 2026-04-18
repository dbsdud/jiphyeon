/**
 * Slice 1.3a — 코드 블록 하이라이팅.
 * spec: docs/specs/spec-github-markdown.md
 *
 * view 페이지에서 동적 import로 로드되어, 이 모듈 자체가 lazy 청크가 된다.
 * 내부의 Prism 언어 번들은 정적 import로 해당 청크에 묶인다.
 */
import Prism from "prismjs";

// clike / javascript / markup / css는 Prism core에 내장되어 있다.
// 프리로드 대상만 명시적으로 import.
import "prismjs/components/prism-c";
import "prismjs/components/prism-cpp";
import "prismjs/components/prism-csharp";
import "prismjs/components/prism-java";
import "prismjs/components/prism-kotlin";
import "prismjs/components/prism-swift";
import "prismjs/components/prism-ruby";
import "prismjs/components/prism-python";
import "prismjs/components/prism-rust";
import "prismjs/components/prism-go";
import "prismjs/components/prism-typescript";
import "prismjs/components/prism-bash";
import "prismjs/components/prism-powershell";
import "prismjs/components/prism-json";
import "prismjs/components/prism-yaml";
import "prismjs/components/prism-toml";
import "prismjs/components/prism-markdown";
import "prismjs/components/prism-sql";
import "prismjs/components/prism-diff";

const SKIP_CLASSES = new Set(["language-mermaid", "language-dbml"]);
const MARKER = "prismHighlighted";

export interface HighlightReport {
  succeeded: number;
  failed: number;
}

/**
 * `root` 내부의 `<pre><code class="language-*">` 블록을 Prism으로 하이라이트한다.
 * - `language-mermaid`/`language-dbml`은 건너뛴다 (다이어그램 단계 소관).
 * - 이미 하이라이트된 블록은 멱등성 마커로 스킵한다.
 */
export function highlightCode(root: HTMLElement): HighlightReport {
  const codes = root.querySelectorAll<HTMLElement>(
    "pre > code[class*='language-']",
  );
  let succeeded = 0;
  let failed = 0;

  for (const el of codes) {
    if (el.dataset[MARKER] === "true") continue;
    if (Array.from(el.classList).some((c) => SKIP_CLASSES.has(c))) continue;
    try {
      Prism.highlightElement(el);
      el.dataset[MARKER] = "true";
      succeeded += 1;
    } catch (err) {
      failed += 1;
      console.warn("[highlightCode]", err);
    }
  }

  return { succeeded, failed };
}
