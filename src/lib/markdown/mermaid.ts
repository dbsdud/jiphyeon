/**
 * Slice 1.3b — Mermaid 다이어그램 렌더.
 * spec: docs/specs/spec-diagrams.md
 *
 * `html[data-theme]` 속성을 읽어 `default`(light) 또는 `dark` 테마로 초기화한다.
 */
import type { StageReport } from "./types";

const SELECTOR = "pre > code.language-mermaid";
const MARKER = "mermaidRendered";

function currentTheme(): "default" | "dark" {
  if (typeof document === "undefined") return "default";
  return document.documentElement.dataset.theme === "dark" ? "dark" : "default";
}

export async function renderMermaid(root: HTMLElement): Promise<StageReport> {
  const codes = root.querySelectorAll<HTMLElement>(SELECTOR);
  if (codes.length === 0) return { succeeded: 0, failed: 0, skipped: true };

  const targets = Array.from(codes).filter((el) => {
    const pre = el.parentElement;
    return pre?.tagName === "PRE" && pre.dataset[MARKER] !== "true";
  });
  if (targets.length === 0) return { succeeded: 0, failed: 0, skipped: true };

  const { default: mermaid } = await import("mermaid");
  mermaid.initialize({
    startOnLoad: false,
    theme: currentTheme(),
    securityLevel: "loose",
    fontFamily: "var(--font-mono)",
  });

  let succeeded = 0;
  let failed = 0;

  for (let i = 0; i < targets.length; i++) {
    const codeEl = targets[i];
    const pre = codeEl.parentElement;
    if (!pre) continue;

    const source = codeEl.textContent ?? "";
    const id = `mermaid-${Date.now()}-${i}`;
    try {
      const { svg } = await mermaid.render(id, source);
      const container = document.createElement("div");
      container.className = "diagram diagram-mermaid";
      container.innerHTML = svg;
      container.dataset[MARKER] = "true";
      pre.replaceWith(container);
      succeeded += 1;
    } catch (err) {
      console.warn("[renderMermaid]", err);
      const overlay = document.createElement("div");
      overlay.className = "diagram-error";
      overlay.title = err instanceof Error ? err.message : String(err);
      overlay.textContent = "Failed to render Mermaid diagram";
      pre.after(overlay);
      pre.dataset[MARKER] = "true";
      failed += 1;
    }
  }

  return { succeeded, failed, skipped: false };
}
