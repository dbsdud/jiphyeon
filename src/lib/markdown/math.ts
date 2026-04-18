/**
 * Slice 1.3b — 수식 렌더 (`$...$`, `$$...$$`).
 * spec: docs/specs/spec-diagrams.md
 *
 * KaTeX와 auto-render extension은 수식 구문이 노트에 실제로 있을 때만
 * 동적 import로 로드되어 별도 청크로 분리된다.
 */
import type { StageReport } from "./types";

const INLINE_MATH = /\$[^$\n]+?\$/;
const BLOCK_MATH = /\$\$[\s\S]+?\$\$/;

function hasMathSyntax(root: HTMLElement): boolean {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode(node) {
      const el = node.parentElement;
      if (!el) return NodeFilter.FILTER_REJECT;
      if (el.closest("code, pre")) return NodeFilter.FILTER_REJECT;
      return NodeFilter.FILTER_ACCEPT;
    },
  });
  while (walker.nextNode()) {
    const text = walker.currentNode.nodeValue ?? "";
    if (BLOCK_MATH.test(text) || INLINE_MATH.test(text)) return true;
  }
  return false;
}

export async function renderMath(root: HTMLElement): Promise<StageReport> {
  if (root.dataset.katexRendered === "true") {
    return { succeeded: 0, failed: 0, skipped: true };
  }
  if (!hasMathSyntax(root)) {
    return { succeeded: 0, failed: 0, skipped: true };
  }

  const [{ default: renderMathInElement }] = await Promise.all([
    import("katex/contrib/auto-render"),
    import("katex/dist/katex.min.css"),
  ]);

  let failed = 0;
  renderMathInElement(root, {
    delimiters: [
      { left: "$$", right: "$$", display: true },
      { left: "$", right: "$", display: false },
    ],
    ignoredTags: ["script", "noscript", "style", "textarea", "pre", "code"],
    throwOnError: false,
    errorCallback: (msg: string, err: unknown) => {
      failed += 1;
      console.warn("[renderMath]", msg, err);
    },
  });

  root.dataset.katexRendered = "true";
  const total = root.querySelectorAll(".katex").length;
  const succeeded = Math.max(0, total - failed);
  return { succeeded, failed, skipped: false };
}
