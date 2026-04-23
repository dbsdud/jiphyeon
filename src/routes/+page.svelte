<script lang="ts">
  import { getVaultStatus } from "$lib/api";
  import { vaultRefresh } from "$lib/stores/vault.svelte";

  let vaultName = $state("");

  function deriveName(path: string | null): string {
    if (!path) return "";
    const trimmed = path.replace(/\/+$/, "");
    const idx = trimmed.lastIndexOf("/");
    return idx >= 0 ? trimmed.slice(idx + 1) : trimmed;
  }

  async function load() {
    const v = await getVaultStatus().catch(() => null);
    vaultName = deriveName(v?.vault_path ?? null);
  }

  $effect(() => {
    vaultRefresh.version;
    load();
  });
</script>

<div class="p-6 max-w-5xl">
  <div class="mb-6">
    <h2 class="text-xl font-semibold">Dashboard</h2>
    {#if vaultName}
      <div class="text-sm text-fg-muted mt-1">📓 {vaultName}</div>
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
