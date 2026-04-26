<script lang="ts">
  import {
    getGraphifyGraph,
    getGraphifyStatus,
    openInEditor,
  } from "$lib/api";
  import type { GraphifyGraph, GraphifyNode, GraphifyStatus } from "$lib/types";
  import LinkGraph from "$lib/components/LinkGraph.svelte";
  import { vaultRefresh } from "$lib/stores/vault.svelte";
  import { goto } from "$app/navigation";

  let status = $state<GraphifyStatus | null>(null);
  let graph = $state<GraphifyGraph | null>(null);
  let loading = $state(true);
  let error = $state("");

  let query = $state("");
  let communityFilter = $state<number | null>(null);

  async function load(): Promise<void> {
    loading = true;
    error = "";
    try {
      status = await getGraphifyStatus();
      if (status.graph_json_exists) {
        graph = await getGraphifyGraph();
      } else {
        graph = null;
      }
    } catch (e) {
      error = String(e);
      graph = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    vaultRefresh.version;
    load();
  });

  const availableCommunities = $derived.by(() => {
    if (!graph) return [];
    const set = new Set<number>();
    for (const n of graph.nodes) {
      if (n.community !== null && n.community !== undefined) set.add(n.community);
    }
    return [...set].sort((a, b) => a - b);
  });

  const activeNodeIds = $derived.by(() => {
    if (!graph) return null;
    const q = query.trim().toLowerCase();
    if (q === "" && communityFilter === null) return null;
    const ids = new Set<string>();
    for (const n of graph.nodes) {
      const labelMatch = q === "" || n.label.toLowerCase().includes(q);
      const commMatch = communityFilter === null || n.community === communityFilter;
      if (labelMatch && commMatch) ids.add(n.id);
    }
    return ids;
  });

  function isMarkdownPath(p: string | null | undefined): boolean {
    return !!p && p.toLowerCase().endsWith(".md");
  }

  async function handleNodeSelect(node: GraphifyNode): Promise<void> {
    const src = node.source_file;
    if (!src) return;
    if (isMarkdownPath(src)) {
      // 활성 프로젝트 docs/ 기준 상대 경로면 그대로, 절대 경로면 그대로 전달.
      goto(`/view?path=${encodeURIComponent(src)}`);
    } else {
      try {
        await openInEditor(src);
      } catch (e) {
        console.warn("openInEditor 실패", e);
      }
    }
  }

  function clearFilters(): void {
    query = "";
    communityFilter = null;
  }
</script>

<div class="h-full flex flex-col">
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
    <div class="flex items-center gap-3">
      <h2 class="text-lg font-semibold">Graphify Graph</h2>
      {#if graph}
        <span class="text-xs text-fg-muted">
          {graph.nodes.length} nodes · {graph.edges.length} edges
          {#if graph.hyperedges.length > 0}
            · {graph.hyperedges.length} hyperedges
          {/if}
        </span>
      {/if}
    </div>
    {#if status?.last_run_at}
      <span class="text-xs text-fg-muted">last run: {new Date(status.last_run_at).toLocaleString()}</span>
    {/if}
  </div>

  {#if graph && graph.nodes.length > 0}
    <div class="flex items-center gap-2 px-4 py-2 border-b border-border shrink-0 bg-surface-1">
      <input
        type="text"
        placeholder="Search label..."
        bind:value={query}
        class="flex-1 min-w-0 px-3 py-1.5 text-sm rounded border border-border bg-surface-0 focus:outline-none focus:border-accent"
      />
      <select
        bind:value={communityFilter}
        class="px-2 py-1.5 text-sm rounded border border-border bg-surface-0 focus:outline-none focus:border-accent"
      >
        <option value={null}>All communities</option>
        {#each availableCommunities as c}
          <option value={c}>Community {c}</option>
        {/each}
      </select>
      <button
        onclick={clearFilters}
        disabled={query === "" && communityFilter === null}
        class="px-3 py-1.5 text-sm rounded border border-border hover:bg-surface-2 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
      >
        Clear
      </button>
    </div>
  {/if}

  <!-- Body -->
  <div class="flex-1 overflow-hidden">
    {#if loading}
      <div class="flex items-center justify-center h-full">
        <p class="text-sm text-fg-muted">Loading graph...</p>
      </div>
    {:else if error}
      <div class="flex items-center justify-center h-full p-6">
        <p class="text-sm text-danger break-all max-w-xl">{error}</p>
      </div>
    {:else if !status?.graph_json_exists}
      <div class="flex items-center justify-center h-full p-8">
        <div class="bg-surface-1 border border-border rounded-xl p-6 max-w-md text-center">
          <p class="text-sm text-fg mb-3">이 프로젝트에서 graphify 가 실행되지 않았습니다.</p>
          {#if status?.graphify_out_path}
            <p class="text-xs text-fg-muted mb-2">
              hub 경로:
              <code class="px-1 py-0.5 rounded bg-surface-2">{status.graphify_out_path}</code>
            </p>
          {/if}
          <p class="text-xs text-fg-muted">
            터미널에서
            <code class="px-1 py-0.5 rounded bg-surface-2">cd ~/Jiphyeon/&lt;project&gt;</code>
            후 Claude Code 에서
            <code class="px-1 py-0.5 rounded bg-surface-2">/graphify</code>
            를 실행하세요.
          </p>
        </div>
      </div>
    {:else if graph && graph.nodes.length > 0}
      <LinkGraph {graph} {activeNodeIds} onSelect={handleNodeSelect} />
    {:else}
      <div class="flex items-center justify-center h-full">
        <p class="text-sm text-fg-muted">No nodes to display.</p>
      </div>
    {/if}
  </div>
</div>
