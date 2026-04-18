/**
 * Slice 1.3b — 마크다운 렌더 파이프라인.
 * spec: docs/specs/spec-diagrams.md
 *
 * 실행 순서: KaTeX → Prism → Mermaid.
 * 각 스테이지는 독립적 try/catch — 하나가 실패해도 다음 스테이지는 정상 진행.
 *
 * DBML은 브라우저 호환 렌더러 부재로 v1.0 MVP에서 제외 (별도 슬라이스에서 탐색).
 */
import { renderMath } from "./math";
import { highlightCode } from "./highlight";
import { renderMermaid } from "./mermaid";
import type { StageReport } from "./types";

export interface PipelineReport {
  math: StageReport;
  highlight: StageReport;
  mermaid: StageReport;
}

const EMPTY: StageReport = { succeeded: 0, failed: 0, skipped: true };

export async function renderMarkdownPipeline(
  root: HTMLElement,
): Promise<PipelineReport> {
  const report: PipelineReport = {
    math: EMPTY,
    highlight: EMPTY,
    mermaid: EMPTY,
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

  return report;
}
