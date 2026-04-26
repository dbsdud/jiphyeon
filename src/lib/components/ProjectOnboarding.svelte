<script lang="ts">
  import { ProjectOnboardingFlow } from "$lib/onboarding-flow.svelte";
  import type { ProjectEntry } from "$lib/types";

  interface Props {
    onconnected: (entry: ProjectEntry) => void;
  }

  let { onconnected }: Props = $props();
  const flow = new ProjectOnboardingFlow((entry) => onconnected(entry));

  const busy = $derived(
    flow.phase === "picking" ||
      flow.phase === "inspecting" ||
      flow.phase === "registering",
  );
</script>

<div class="min-h-screen flex items-center justify-center p-8">
  <div class="bg-surface-1 border border-border rounded-xl max-w-md w-full p-8 text-center">
    <h1 class="text-2xl font-bold mb-3">집현 v2.0</h1>
    <p class="text-sm text-fg-muted mb-6">
      프로젝트(레포) 폴더를 등록하면 집현이 해당 프로젝트의 docs/ 와 graphify-out/ 을 데이터 소스로 사용합니다.
    </p>

    {#if flow.phase === "decision" && flow.inspection}
      <div class="text-left bg-surface-2 border border-border rounded-lg p-4 mb-4">
        <div class="text-xs text-fg-muted mb-1">선택한 폴더</div>
        <div class="font-mono text-sm text-fg break-all mb-3">{flow.inspection.root_path}</div>

        {#if flow.inspection.already_registered}
          <p class="text-xs text-warning mb-3">
            이미 등록된 프로젝트입니다. 활성 프로젝트로 전환만 됩니다.
          </p>
        {/if}

        <p class="text-sm text-fg mb-4">
          <code class="text-xs px-1 py-0.5 rounded bg-surface-3">docs/</code>
          폴더가 없습니다. 자동으로 생성하시겠습니까?
        </p>

        <div class="flex gap-2 justify-end">
          <button
            class="px-3 py-1.5 text-sm rounded border border-border text-fg-muted hover:text-fg disabled:opacity-50"
            onclick={() => flow.cancelDecision()}
            disabled={busy}
          >
            취소
          </button>
          <button
            class="px-3 py-1.5 text-sm rounded bg-accent text-accent-fg hover:bg-accent/80 disabled:opacity-50"
            onclick={() => flow.confirmRegister(true)}
            disabled={busy}
          >
            {busy ? "등록 중..." : "생성 후 등록"}
          </button>
        </div>
      </div>
    {:else}
      <button
        class="px-5 py-2 rounded bg-accent text-accent-fg hover:bg-accent/80 transition-colors disabled:opacity-50"
        onclick={() => flow.pickAndInspect()}
        disabled={busy}
      >
        {busy ? "확인 중..." : "프로젝트 폴더 선택"}
      </button>
    {/if}

    {#if flow.inspection && !flow.inspection.graphify_out_exists && flow.phase !== "done"}
      <p class="text-xs text-fg-muted/70 mt-4">
        <code class="text-xs px-1 py-0.5 rounded bg-surface-3">graphify-out/</code>
        이 없습니다. 프로젝트를 Claude Code 에서 열고
        <code class="text-xs px-1 py-0.5 rounded bg-surface-3">/graphify</code>
        를 먼저 실행하세요.
      </p>
    {/if}

    {#if flow.error}
      <p class="text-xs text-danger mt-3 break-all">{flow.error}</p>
    {/if}
  </div>
</div>
