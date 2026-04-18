/**
 * Slice 1.3b — DBML 스키마 다이어그램 렌더.
 * spec: docs/specs/spec-diagrams.md
 */
import type { StageReport } from "./types";

const SELECTOR = "pre > code.language-dbml";
const MARKER = "dbmlRendered";

export async function renderDbml(root: HTMLElement): Promise<StageReport> {
  const codes = root.querySelectorAll<HTMLElement>(SELECTOR);
  if (codes.length === 0) return { succeeded: 0, failed: 0, skipped: true };

  const targets = Array.from(codes).filter((el) => {
    const pre = el.parentElement;
    return pre?.tagName === "PRE" && pre.dataset[MARKER] !== "true";
  });
  if (targets.length === 0) return { succeeded: 0, failed: 0, skipped: true };

  const { run } = await import("@softwaretechnik/dbml-renderer");

  let succeeded = 0;
  let failed = 0;

  for (const codeEl of targets) {
    const pre = codeEl.parentElement!;
    const source = (codeEl.textContent ?? "").trim();
    if (!source) {
      pre.dataset[MARKER] = "true";
      continue;
    }

    try {
      const svg = run(source, "svg");
      const container = document.createElement("div");
      container.className = "diagram diagram-dbml";
      container.innerHTML = svg;
      container.dataset[MARKER] = "true";
      pre.replaceWith(container);
      succeeded += 1;
    } catch (err) {
      console.warn("[renderDbml]", err);
      const overlay = document.createElement("div");
      overlay.className = "diagram-error";
      overlay.title = err instanceof Error ? err.message : String(err);
      overlay.textContent = "Failed to render DBML";
      pre.after(overlay);
      pre.dataset[MARKER] = "true";
      failed += 1;
    }
  }

  return { succeeded, failed, skipped: false };
}
