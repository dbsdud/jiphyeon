/**
 * 프로젝트 등록 온보딩의 상태 머신.
 *
 * 흐름: idle → picking → inspecting → decision → registering → done
 *                              ↓             ↓           ↓
 *                            error         error       error
 *
 * ProjectOnboarding.svelte 와 AddProjectModal.svelte 가 인스턴스를 각각 생성해 사용.
 */

import { open as openDialog } from "@tauri-apps/plugin-dialog";

import {
  inspectProjectRoot,
  registerProject,
} from "./api";
import type { ProjectEntry, ProjectInspection } from "./types";

export type OnboardingPhase =
  | "idle"
  | "picking"
  | "inspecting"
  | "decision"
  | "registering"
  | "done";

export class ProjectOnboardingFlow {
  phase = $state<OnboardingPhase>("idle");
  inspection = $state<ProjectInspection | null>(null);
  error = $state("");
  registered = $state<ProjectEntry | null>(null);

  private readonly onComplete: (entry: ProjectEntry) => void;

  constructor(onComplete: (entry: ProjectEntry) => void) {
    this.onComplete = onComplete;
  }

  reset(): void {
    this.phase = "idle";
    this.inspection = null;
    this.error = "";
    this.registered = null;
  }

  /** 사용자가 "폴더 선택" 버튼을 눌렀을 때. */
  async pickAndInspect(): Promise<void> {
    this.error = "";
    this.phase = "picking";
    let picked: string | string[] | null;
    try {
      picked = await openDialog({ directory: true, multiple: false });
    } catch (e) {
      this.error = String(e);
      this.phase = "idle";
      return;
    }
    if (typeof picked !== "string") {
      this.phase = "idle";
      return;
    }

    this.phase = "inspecting";
    let result: ProjectInspection;
    try {
      result = await inspectProjectRoot(picked);
    } catch (e) {
      this.error = String(e);
      this.phase = "idle";
      return;
    }

    this.inspection = result;

    if (!result.root_exists) {
      this.error = `폴더를 찾을 수 없습니다: ${result.root_path}`;
      this.phase = "idle";
      return;
    }
    if (result.docs_exists && !result.docs_is_dir) {
      this.error = `docs/ 가 폴더가 아닙니다: ${result.root_path}/docs`;
      this.phase = "idle";
      return;
    }

    if (result.docs_exists) {
      // docs/ 존재 → 다이얼로그 없이 즉시 등록
      await this.confirmRegister(false);
    } else {
      // docs/ 없음 → 사용자 결정 대기
      this.phase = "decision";
    }
  }

  /** decision 단계에서 사용자가 등록 버튼을 눌렀을 때. */
  async confirmRegister(createDocs: boolean): Promise<void> {
    if (!this.inspection) return;
    this.phase = "registering";
    this.error = "";
    try {
      const entry = await registerProject(this.inspection.root_path, null, createDocs);
      this.registered = entry;
      this.phase = "done";
      this.onComplete(entry);
    } catch (e) {
      this.error = String(e);
      this.phase = "decision";
    }
  }

  /** 다이얼로그 취소 → idle 로 복귀. */
  cancelDecision(): void {
    this.phase = "idle";
    this.inspection = null;
  }
}
