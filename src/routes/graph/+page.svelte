<script lang="ts">
  import { getLinkGraph } from "$lib/api";
  import type { LinkGraph } from "$lib/types";
  import LinkGraphComponent from "$lib/components/LinkGraph.svelte";

  let graph = $state<LinkGraph | null>(null);
  let loading = $state(true);
  let error = $state("");

  const typeColors: Record<string, string> = {
    til: "#3b82f6",
    decision: "#eab308",
    reading: "#22c55e",
    meeting: "#a78bfa",
    idea: "#f59e0b",
    artifact: "#ec4899",
    clipping: "#06b6d4",
    moc: "#f97316",
  };

  async function load() {
    try {
      graph = await getLinkGraph();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  load();
</script>

<div class="h-full flex flex-col">
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
    <div class="flex items-center gap-3">
      <h2 class="text-lg font-semibold">Link Graph</h2>
      {#if graph}
        <span class="text-xs text-muted">{graph.nodes.length} nodes, {graph.edges.length} edges</span>
      {/if}
    </div>
    <!-- Legend -->
    <div class="flex items-center gap-3">
      {#each Object.entries(typeColors) as [type, color]}
        <div class="flex items-center gap-1">
          <span class="w-2.5 h-2.5 rounded-full" style="background: {color}"></span>
          <span class="text-xs text-muted">{type}</span>
        </div>
      {/each}
    </div>
  </div>

  <!-- Graph -->
  <div class="flex-1 overflow-hidden">
    {#if loading}
      <div class="flex items-center justify-center h-full">
        <p class="text-sm text-muted">Loading graph...</p>
      </div>
    {:else if error}
      <div class="flex items-center justify-center h-full">
        <p class="text-sm text-danger">{error}</p>
      </div>
    {:else if graph && graph.nodes.length > 0}
      <LinkGraphComponent {graph} />
    {:else}
      <div class="flex items-center justify-center h-full">
        <p class="text-sm text-muted">No nodes to display.</p>
      </div>
    {/if}
  </div>
</div>
