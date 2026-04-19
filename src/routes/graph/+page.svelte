<script lang="ts">
  import { getLinkGraph } from "$lib/api";
  import type { LinkGraph } from "$lib/types";
  import LinkGraphComponent from "$lib/components/LinkGraph.svelte";
  import { vaultRefresh } from "$lib/stores/vault.svelte";
  import {
    computeActiveIds,
    emptyFilter,
    isFilterEmpty,
    type GraphFilter,
  } from "$lib/graph-filter";

  let graph = $state<LinkGraph | null>(null);
  let loading = $state(true);
  let error = $state("");
  let filter = $state<GraphFilter>({ ...emptyFilter });

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

  const availableTypes = $derived(
    graph
      ? Array.from(
          new Set(
            graph.nodes
              .map((n) => n.note_type)
              .filter((t): t is string => !!t),
          ),
        ).sort()
      : [],
  );

  const availableTags = $derived(
    graph
      ? Array.from(new Set(graph.nodes.flatMap((n) => n.tags))).sort()
      : [],
  );

  const activeNodeIds = $derived(
    graph && !isFilterEmpty(filter)
      ? computeActiveIds(graph.nodes, graph.edges, filter)
      : null,
  );

  async function load() {
    try {
      graph = await getLinkGraph();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    vaultRefresh.version;
    load();
  });

  function clearFilter() {
    filter = { ...emptyFilter };
  }
</script>

<div class="h-full flex flex-col">
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
    <div class="flex items-center gap-3">
      <h2 class="text-lg font-semibold">Link Graph</h2>
      {#if graph}
        <span class="text-xs text-fg-muted">{graph.nodes.length} nodes, {graph.edges.length} edges</span>
      {/if}
    </div>
    <!-- Legend -->
    <div class="flex items-center gap-3">
      {#each Object.entries(typeColors) as [type, color]}
        <div class="flex items-center gap-1">
          <span class="w-2.5 h-2.5 rounded-full" style="background: {color}"></span>
          <span class="text-xs text-fg-muted">{type}</span>
        </div>
      {/each}
    </div>
  </div>

  <!-- Search / filter bar -->
  {#if graph && graph.nodes.length > 0}
    <div class="flex items-center gap-2 px-4 py-2 border-b border-border shrink-0 bg-surface-1">
      <input
        type="text"
        placeholder="Search title..."
        bind:value={filter.query}
        class="flex-1 min-w-0 px-3 py-1.5 text-sm rounded border border-border bg-surface-0 focus:outline-none focus:border-accent"
      />
      <select
        bind:value={filter.typeFilter}
        class="px-2 py-1.5 text-sm rounded border border-border bg-surface-0 focus:outline-none focus:border-accent"
      >
        <option value={null}>All types</option>
        {#each availableTypes as t}
          <option value={t}>{t}</option>
        {/each}
      </select>
      <select
        bind:value={filter.tagFilter}
        class="px-2 py-1.5 text-sm rounded border border-border bg-surface-0 focus:outline-none focus:border-accent"
      >
        <option value={null}>All tags</option>
        {#each availableTags as t}
          <option value={t}>{t}</option>
        {/each}
      </select>
      <button
        onclick={clearFilter}
        disabled={isFilterEmpty(filter)}
        class="px-3 py-1.5 text-sm rounded border border-border hover:bg-surface-2 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
      >
        Clear
      </button>
    </div>
  {/if}

  <!-- Graph -->
  <div class="flex-1 overflow-hidden">
    {#if loading}
      <div class="flex items-center justify-center h-full">
        <p class="text-sm text-fg-muted">Loading graph...</p>
      </div>
    {:else if error}
      <div class="flex items-center justify-center h-full">
        <p class="text-sm text-danger">{error}</p>
      </div>
    {:else if graph && graph.nodes.length > 0}
      <LinkGraphComponent {graph} {activeNodeIds} />
    {:else}
      <div class="flex items-center justify-center h-full">
        <p class="text-sm text-fg-muted">No nodes to display.</p>
      </div>
    {/if}
  </div>
</div>
