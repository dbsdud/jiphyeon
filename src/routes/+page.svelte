<script lang="ts">
  import { getActiveProject } from "$lib/api";
  import { vaultRefresh } from "$lib/stores/vault.svelte";

  let projectName = $state("");

  async function load(): Promise<void> {
    const p = await getActiveProject().catch(() => null);
    projectName = p?.name ?? "";
  }

  $effect(() => {
    vaultRefresh.version;
    load();
  });
</script>

<div class="p-6 max-w-5xl">
  <div class="mb-6">
    <h2 class="text-xl font-semibold">Dashboard</h2>
    {#if projectName}
      <div class="text-sm text-fg-muted mt-1">📁 {projectName}</div>
    {/if}
  </div>

  <div class="bg-surface-1 rounded-lg border border-border p-8 text-center">
    <p class="text-sm text-fg-muted">
      v2.0 대시보드는 graphify 출력 기반으로 재설계 중입니다.
    </p>
    <p class="text-xs text-fg-muted mt-2">
      Epic C에서 graph.json / GRAPH_REPORT 시각화 카드로 복원 예정.
    </p>
  </div>
</div>
