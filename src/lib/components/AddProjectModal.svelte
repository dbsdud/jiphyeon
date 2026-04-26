<script lang="ts">
  import { ProjectOnboardingFlow } from "$lib/onboarding-flow.svelte";
  import type { ProjectEntry } from "$lib/types";

  interface Props {
    open: boolean;
    onclose: () => void;
    onadded: (entry: ProjectEntry) => void;
  }

  let { open, onclose, onadded }: Props = $props();

  function handleComplete(entry: ProjectEntry): void {
    onadded(entry);
    onclose();
    flow.reset();
  }

  const flow = new ProjectOnboardingFlow(handleComplete);

  const busy = $derived(
    flow.phase === "picking" ||
      flow.phase === "inspecting" ||
      flow.phase === "registering",
  );

  function close(): void {
    if (busy) return;
    flow.reset();
    onclose();
  }
</script>

{#if open}
  <div class="fixed inset-0 bg-black/40 z-40 flex items-center justify-center p-6">
    <div class="bg-surface-1 border border-border rounded-xl w-full max-w-md p-6">
      <h3 class="text-lg font-semibold mb-3">프로젝트 추가</h3>

      {#if flow.phase === "decision" && flow.inspection}
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
      {:else}
        <p class="text-sm text-fg-muted mb-4">
          등록할 프로젝트 폴더를 선택하세요.
        </p>
        <div class="flex gap-2 justify-end">
          <button
            class="px-3 py-1.5 text-sm rounded border border-border text-fg-muted hover:text-fg disabled:opacity-50"
            onclick={close}
            disabled={busy}
          >
            취소
          </button>
          <button
            class="px-3 py-1.5 text-sm rounded bg-accent text-accent-fg hover:bg-accent/80 disabled:opacity-50"
            onclick={() => flow.pickAndInspect()}
            disabled={busy}
          >
            {busy ? "확인 중..." : "폴더 선택"}
          </button>
        </div>
      {/if}

      {#if flow.inspection && !flow.inspection.graphify_out_exists && flow.phase !== "done"}
        <p class="text-xs text-fg-muted/70 mt-4">
          <code class="text-xs px-1 py-0.5 rounded bg-surface-3">graphify-out/</code>
          이 없습니다. Claude Code 에서
          <code class="text-xs px-1 py-0.5 rounded bg-surface-3">/graphify</code>
          를 먼저 실행하세요.
        </p>
      {/if}

      {#if flow.error}
        <p class="text-xs text-danger mt-3 break-all">{flow.error}</p>
      {/if}
    </div>
  </div>
{/if}
