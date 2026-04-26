<script lang="ts">
  import {
    getCrossProjectGraph,
    getGraphifyGraph,
    getGraphifyStatus,
    listProjects,
    openInEditor,
    switchProject,
  } from "$lib/api";
  import type {
    CrossProjectGraph,
    GraphifyGraph,
    GraphifyStatus,
    ProjectEntry,
  } from "$lib/types";
  import LinkGraph from "$lib/components/LinkGraph.svelte";
  import type { GraphView } from "$lib/components/LinkGraph.svelte";
  import { adaptCross, adaptSingle } from "$lib/graph-adapter";
  import { vaultRefresh } from "$lib/stores/vault.svelte";
  import { goto } from "$app/navigation";

  type Mode = "single" | "cross";
  let mode = $state<Mode>("single");

  // Single mode
  let status = $state<GraphifyStatus | null>(null);
  let singleGraph = $state<GraphifyGraph | null>(null);

  // Cross mode
  let projects = $state<ProjectEntry[]>([]);
  let selectedProjectIds = $state<Set<string>>(new Set());
  let mergeLabels = $state(true);
  let crossGraph = $state<CrossProjectGraph | null>(null);
  let crossError = $state("");

  let loading = $state(true);
  let error = $state("");
  let query = $state("");
  let communityFilter = $state<number | null>(null);

  async function loadSingle(): Promise<void> {
    loading = true;
    error = "";
    try {
      status = await getGraphifyStatus();
      singleGraph = status.graph_json_exists ? await getGraphifyGraph() : null;
    } catch (e) {
      error = String(e);
      singleGraph = null;
    } finally {
      loading = false;
    }
  }

  async function loadProjectsList(): Promise<void> {
    projects = await listProjects().catch(() => []);
    if (selectedProjectIds.size === 0) {
      selectedProjectIds = new Set(projects.map((p) => p.id));
    }
  }

  async function loadCross(): Promise<void> {
    loading = true;
    crossError = "";
    crossGraph = null;
    if (selectedProjectIds.size === 0) {
      loading = false;
      return;
    }
    try {
      crossGraph = await getCrossProjectGraph([...selectedProjectIds], mergeLabels);
    } catch (e) {
      crossError = String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    vaultRefresh.version;
    loadProjectsList();
    if (mode === "single") {
      loadSingle();
    } else {
      loadCross();
    }
  });

  // mode 또는 cross 옵션이 바뀌면 cross 재로드
  $effect(() => {
    if (mode !== "cross") return;
    void selectedProjectIds;
    void mergeLabels;
    loadCross();
  });

  const view: GraphView | null = $derived.by(() => {
    if (mode === "single") {
      return singleGraph ? adaptSingle(singleGraph) : null;
    }
    return crossGraph ? adaptCross(crossGraph) : null;
  });

  const availableCommunities = $derived.by(() => {
    if (!view) return [];
    const set = new Set<number>();
    for (const n of view.nodes) {
      if (n.community !== null) set.add(n.community);
    }
    return [...set].sort((a, b) => a - b);
  });

  const activeNodeIds = $derived.by(() => {
    if (!view) return null;
    const q = query.trim().toLowerCase();
    if (q === "" && communityFilter === null) return null;
    const ids = new Set<string>();
    for (const n of view.nodes) {
      const labelMatch = q === "" || n.label.toLowerCase().includes(q);
      const commMatch = communityFilter === null || n.community === communityFilter;
      if (labelMatch && commMatch) ids.add(n.id);
    }
    return ids;
  });

  function toggleProject(id: string): void {
    const next = new Set(selectedProjectIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selectedProjectIds = next;
  }

  function selectAllProjects(): void {
    selectedProjectIds = new Set(projects.map((p) => p.id));
  }

  function clearProjects(): void {
    selectedProjectIds = new Set();
  }

  function clearFilters(): void {
    query = "";
    communityFilter = null;
  }

  function isMarkdownPath(p: string | null | undefined): boolean {
    return !!p && p.toLowerCase().endsWith(".md");
  }

  async function handleSingleNodeSelect(nodeId: string): Promise<void> {
    const node = singleGraph?.nodes.find((n) => n.id === nodeId);
    const src = node?.source_file ?? null;
    if (!src) return;
    if (isMarkdownPath(src)) {
      goto(`/view?path=${encodeURIComponent(src)}`);
    } else {
      try {
        await openInEditor(src);
      } catch (e) {
        console.warn("openInEditor 실패", e);
      }
    }
  }

  async function handleCrossNodeSelect(nodeId: string): Promise<void> {
    const node = crossGraph?.nodes.find((n) => n.id === nodeId);
    if (!node) return;
    try {
      await switchProject(node.project_id);
      // 프로젝트 전환 후 single 모드로 reload
      mode = "single";
      window.location.reload();
    } catch (e) {
      console.warn("switchProject 실패", e);
    }
  }

  function handleNodeSelect(nodeId: string): void {
    if (mode === "single") {
      handleSingleNodeSelect(nodeId);
    } else {
      handleCrossNodeSelect(nodeId);
    }
  }
</script>

<div class="h-full flex flex-col">
  <!-- Header -->
  <div class="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
    <div class="flex items-center gap-3">
      <h2 class="text-lg font-semibold">Graphify Graph</h2>
      {#if view}
        <span class="text-xs text-fg-muted">
          {view.nodes.length} nodes · {view.edges.length} edges
          {#if mode === "cross" && crossGraph}
            · {crossGraph.members.length} projects
            · {crossGraph.edges.filter((e) => e.is_bridge).length} bridges
          {/if}
        </span>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <div class="flex rounded border border-border overflow-hidden">
        <button
          class="px-3 py-1 text-xs {mode === 'single' ? 'bg-accent text-accent-fg' : 'text-fg-muted hover:bg-surface-2'}"
          onclick={() => { mode = "single"; }}
        >
          단일
        </button>
        <button
          class="px-3 py-1 text-xs {mode === 'cross' ? 'bg-accent text-accent-fg' : 'text-fg-muted hover:bg-surface-2'}"
          onclick={() => { mode = "cross"; }}
        >
          전체 프로젝트
        </button>
      </div>
      {#if mode === "single" && status?.last_run_at}
        <span class="text-xs text-fg-muted">last run: {new Date(status.last_run_at).toLocaleString()}</span>
      {/if}
    </div>
  </div>

  {#if mode === "cross"}
    <div class="flex flex-wrap items-center gap-2 px-4 py-2 border-b border-border shrink-0 bg-surface-1">
      <span class="text-xs text-fg-muted shrink-0">프로젝트:</span>
      {#each projects as p}
        {@const checked = selectedProjectIds.has(p.id)}
        <button
          class="text-xs px-2 py-0.5 rounded-full border transition-colors
            {checked ? 'bg-accent text-accent-fg border-accent' : 'border-border text-fg-muted hover:border-accent'}"
          onclick={() => toggleProject(p.id)}
        >
          {p.name}
        </button>
      {/each}
      <button
        class="text-xs text-fg-muted hover:text-fg ml-2"
        onclick={selectAllProjects}
      >
        전체
      </button>
      <button
        class="text-xs text-fg-muted hover:text-fg"
        onclick={clearProjects}
      >
        해제
      </button>
      <label class="ml-3 flex items-center gap-1 text-xs text-fg-muted cursor-pointer">
        <input type="checkbox" bind:checked={mergeLabels} />
        라벨 병합 (브리지)
      </label>
    </div>
  {/if}

  {#if view && view.nodes.length > 0}
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
    {:else if mode === "single"}
      {#if error}
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
      {:else if view && view.nodes.length > 0}
        <LinkGraph graph={view} {activeNodeIds} onSelect={handleNodeSelect} />
      {:else}
        <div class="flex items-center justify-center h-full">
          <p class="text-sm text-fg-muted">No nodes to display.</p>
        </div>
      {/if}
    {:else if crossError}
      <div class="flex items-center justify-center h-full p-6">
        <p class="text-sm text-danger break-all max-w-xl">{crossError}</p>
      </div>
    {:else if selectedProjectIds.size === 0}
      <div class="flex items-center justify-center h-full p-6">
        <p class="text-sm text-fg-muted">표시할 프로젝트를 선택하세요.</p>
      </div>
    {:else if view && view.nodes.length > 0}
      <LinkGraph graph={view} {activeNodeIds} onSelect={handleNodeSelect} />
    {:else}
      <div class="flex items-center justify-center h-full p-8">
        <div class="bg-surface-1 border border-border rounded-xl p-6 max-w-md text-center">
          <p class="text-sm text-fg-muted">
            선택된 프로젝트 중 graphify 가 실행된 항목이 없습니다.
          </p>
        </div>
      </div>
    {/if}
  </div>
</div>
