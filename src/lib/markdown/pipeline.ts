/**
 * Slice 1.3b — 마크다운 렌더 파이프라인.
 * spec: docs/specs/spec-diagrams.md
 *
 * 실행 순서: KaTeX → Prism → Mermaid → DBML.
 * 각 스테이지는 독립적 try/catch — 하나가 실패해도 다음 스테이지는 정상 진행.
 */
import { renderMath } from "./math";
import { highlightCode } from "./highlight";
import { renderMermaid } from "./mermaid";
import { renderDbml } from "./dbml";
import type { StageReport } from "./types";

export interface PipelineReport {
  math: StageReport;
  highlight: StageReport;
  mermaid: StageReport;
  dbml: StageReport;
}

const EMPTY: StageReport = { succeeded: 0, failed: 0, skipped: true };

export async function renderMarkdownPipeline(
  root: HTMLElement,
): Promise<PipelineReport> {
  const report: PipelineReport = {
    math: EMPTY,
    highlight: EMPTY,
    mermaid: EMPTY,
    dbml: EMPTY,
  };

  try {
    report.math = await renderMath(root);
  } catch (err) {
    console.warn("[pipeline.math]", err);
  }

  try {
    const r = highlightCode(root);
    report.highlight = {
      succeeded: r.succeeded,
      failed: r.failed,
      skipped: r.succeeded === 0 && r.failed === 0,
    };
  } catch (err) {
    console.warn("[pipeline.highlight]", err);
  }

  try {
    report.mermaid = await renderMermaid(root);
  } catch (err) {
    console.warn("[pipeline.mermaid]", err);
  }

  try {
    report.dbml = await renderDbml(root);
  } catch (err) {
    console.warn("[pipeline.dbml]", err);
  }

  return report;
}
