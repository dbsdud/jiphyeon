<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import {
    listProjects,
    openInEditor,
    searchAll,
    switchProject,
  } from "$lib/api";
  import type { ProjectEntry, SearchHit, SearchKind } from "$lib/types";
  import { vaultRefresh } from "$lib/stores/vault.svelte";

  let query = $state("");
  let projects = $state<ProjectEntry[]>([]);
  let selectedProjectIds = $state<Set<string>>(new Set());
  let kindFilters = $state<Set<SearchKind>>(new Set());
  let hits = $state<SearchHit[]>([]);
  let loading = $state(false);
  let error = $state("");

  const ALL_KINDS: SearchKind[] = ["file", "node", "qa"];

  onMount(async () => {
    projects = await listProjects().catch(() => []);
    const initial = page.url.searchParams.get("q") ?? "";
    if (initial) {
      query = initial;
      runSearch();
    }
  });

  $effect(() => {
    vaultRefresh.version;
    if (query.trim()) runSearch();
  });

  async function runSearch(): Promise<void> {
    loading = true;
    error = "";
    try {
      const projectFilter = selectedProjectIds.size > 0 ? [...selectedProjectIds] : null;
      const kindFilter = kindFilters.size > 0 ? [...kindFilters] : null;
      hits = await searchAll(query.trim(), projectFilter, kindFilter, 50);
    } catch (e) {
      error = String(e);
      hits = [];
    } finally {
      loading = false;
    }
  }

  function toggleProject(id: string): void {
    const next = new Set(selectedProjectIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selectedProjectIds = next;
    if (query.trim()) runSearch();
  }

  function toggleKind(k: SearchKind): void {
    const next = new Set(kindFilters);
    if (next.has(k)) next.delete(k);
    else next.add(k);
    kindFilters = next;
    if (query.trim()) runSearch();
  }

  function kindIcon(k: SearchKind): string {
    return k === "file" ? "📄" : k === "node" ? "🧩" : "💬";
  }

  function kindLabel(k: SearchKind): string {
    return k === "file" ? "File" : k === "node" ? "Node" : "Q&A";
  }

  async function openHit(h: SearchHit): Promise<void> {
    try {
      // 다른 프로젝트면 switch 후 reload
      if (h.kind === "file") {
        await switchProject(h.project_id);
        await goto(`/view?path=${encodeURIComponent(h.path)}`);
        // 활성 변경 사이드 이펙트 반영
        window.location.reload();
      } else if (h.kind === "node") {
        await switchProject(h.project_id);
        await goto("/graph");
        window.location.reload();
      } else {
        // Qa: graphify-out/memory/<file>.md 의 절대경로를 외부 에디터로
        const proj = projects.find((p) => p.id === h.project_id);
        if (proj) {
          const abs = `${proj.graphify_out_path}/memory/${h.path}`;
          await openInEditor(abs);
        }
      }
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="h-full flex flex-col">
  <div class="px-4 py-3 border-b border-border shrink-0 space-y-2">
    <div class="flex items-center gap-2">
      <input
        type="search"
        placeholder="Search across projects..."
        bind:value={query}
        onkeydown={(e) => { if (e.key === "Enter") runSearch(); }}
        class="flex-1 px-3 py-2 text-sm rounded border border-border bg-surface-0 focus:outline-none focus:border-accent"
      />
      <button
        class="px-3 py-2 text-sm rounded bg-accent text-accent-fg hover:bg-accent/80 disabled:opacity-50"
        onclick={runSearch}
        disabled={!query.trim() || loading}
      >
        {loading ? "Searching..." : "Search"}
      </button>
    </div>

    <div class="flex flex-wrap items-center gap-2 text-xs">
      <span class="text-fg-muted shrink-0">프로젝트:</span>
      {#if projects.length === 0}
        <span class="text-fg-muted">등록된 프로젝트 없음</span>
      {/if}
      {#each projects as p}
        {@const checked = selectedProjectIds.has(p.id)}
        <button
          class="px-2 py-0.5 rounded-full border transition-colors
            {checked ? 'bg-accent text-accent-fg border-accent' : 'border-border text-fg-muted hover:border-accent'}"
          onclick={() => toggleProject(p.id)}
        >
          {p.name}
        </button>
      {/each}
      <span class="ml-3 text-fg-muted shrink-0">종류:</span>
      {#each ALL_KINDS as k}
        {@const checked = kindFilters.has(k)}
        <button
          class="px-2 py-0.5 rounded-full border transition-colors
            {checked ? 'bg-accent text-accent-fg border-accent' : 'border-border text-fg-muted hover:border-accent'}"
          onclick={() => toggleKind(k)}
        >
          {kindIcon(k)} {kindLabel(k)}
        </button>
      {/each}
    </div>
  </div>

  <div class="flex-1 overflow-y-auto">
    {#if error}
      <div class="m-4 bg-danger/10 border border-danger/30 rounded-lg p-3 text-sm text-danger break-all">
        {error}
      </div>
    {:else if loading}
      <p class="p-4 text-sm text-fg-muted">Searching...</p>
    {:else if !query.trim()}
      <p class="p-4 text-sm text-fg-muted">검색어를 입력하세요. 프로젝트/종류 필터로 좁힐 수 있습니다.</p>
    {:else if hits.length === 0}
      <p class="p-4 text-sm text-fg-muted">결과 없음.</p>
    {:else}
      <div class="divide-y divide-border">
        {#each hits as h}
          <button
            type="button"
            onclick={() => openHit(h)}
            class="w-full text-left px-4 py-3 hover:bg-surface-1 transition-colors"
          >
            <div class="flex items-center gap-2 mb-1">
              <span class="text-xs px-1.5 py-0.5 rounded bg-surface-2 text-fg-muted shrink-0">
                📁 {h.project_name}
              </span>
              <span class="text-xs text-fg-muted shrink-0">{kindIcon(h.kind)} {kindLabel(h.kind)}</span>
              <span class="text-sm truncate">{h.title}</span>
              <span class="ml-auto text-xs text-fg-muted shrink-0 tabular-nums">{h.score.toFixed(2)}</span>
            </div>
            {#if h.snippet}
              <p class="text-xs text-fg-muted line-clamp-2">{h.snippet}</p>
            {/if}
            <p class="text-xs text-fg-muted/70 mt-1 truncate">{h.path}</p>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>
