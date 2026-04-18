/**
 * 공통 스테이지 리포트 — 마크다운 렌더 파이프라인 각 단계의 결과 요약.
 * spec: docs/specs/spec-diagrams.md
 */
export interface StageReport {
  succeeded: number;
  failed: number;
  skipped: boolean;
}
