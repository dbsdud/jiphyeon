<script lang="ts">
  import { tick } from "svelte";
  import { goto } from "$app/navigation";
  import { searchAll, switchProject, openInEditor, listProjects } from "$lib/api";
  import type { ProjectEntry, SearchHit, SearchKind } from "$lib/types";

  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let { open, onclose }: Props = $props();

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let activeIndex = $state(0);
  let loading = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);
  let projects = $state<ProjectEntry[]>([]);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  async function ensureProjects(): Promise<void> {
    if (projects.length === 0) {
      projects = await listProjects().catch(() => []);
    }
  }

  $effect(() => {
    if (open) {
      void ensureProjects();
      tick().then(() => inputEl?.focus());
    } else {
      query = "";
      hits = [];
      activeIndex = 0;
      if (debounceTimer) {
        clearTimeout(debounceTimer);
        debounceTimer = null;
      }
    }
  });

  function scheduleSearch(): void {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      runSearch();
    }, 180);
  }

  async function runSearch(): Promise<void> {
    const q = query.trim();
    if (!q) {
      hits = [];
      return;
    }
    loading = true;
    try {
      hits = await searchAll(q, null, null, 10);
      activeIndex = 0;
    } catch {
      hits = [];
    } finally {
      loading = false;
    }
  }

  function kindIcon(k: SearchKind): string {
    return k === "file" ? "📄" : k === "node" ? "🧩" : "💬";
  }

  function moveCursor(delta: number): void {
    if (hits.length === 0) return;
    activeIndex = (activeIndex + delta + hits.length) % hits.length;
  }

  async function activate(h: SearchHit): Promise<void> {
    onclose();
    try {
      if (h.kind === "file") {
        await switchProject(h.project_id);
        await goto(`/view?path=${encodeURIComponent(h.path)}`);
        window.location.reload();
      } else if (h.kind === "node") {
        await switchProject(h.project_id);
        await goto("/graph");
        window.location.reload();
      } else {
        const proj = projects.find((p) => p.id === h.project_id);
        if (proj) {
          await openInEditor(`${proj.graphify_out_path}/memory/${h.path}`);
        }
      }
    } catch (e) {
      console.warn("activate failed", e);
    }
  }

  function viewAllResults(): void {
    if (!query.trim()) return;
    onclose();
    goto(`/search?q=${encodeURIComponent(query.trim())}`);
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      onclose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      moveCursor(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      moveCursor(-1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (e.metaKey || e.ctrlKey) {
        viewAllResults();
      } else if (hits[activeIndex]) {
        activate(hits[activeIndex]);
      }
    }
  }
</script>

{#if open}
  <div
    class="fixed inset-0 bg-black/40 z-40"
    onclick={onclose}
    onkeydown={() => {}}
    role="presentation"
  ></div>
  <div class="fixed top-24 left-1/2 -translate-x-1/2 z-50 w-[640px] max-w-[90vw]">
    <div class="bg-surface-1 border border-border rounded-xl shadow-2xl overflow-hidden">
      <input
        bind:this={inputEl}
        bind:value={query}
        oninput={scheduleSearch}
        onkeydown={onKeydown}
        type="search"
        placeholder="모든 프로젝트에서 검색..."
        class="w-full px-4 py-3 text-sm bg-surface-0 text-fg placeholder:text-fg-muted focus:outline-none border-b border-border"
      />
      {#if loading}
        <p class="px-4 py-3 text-xs text-fg-muted">Searching...</p>
      {:else if !query.trim()}
        <p class="px-4 py-3 text-xs text-fg-muted">키워드를 입력하세요. ⏎ 선택 · ⌘⏎ 전체 결과 보기 · Esc 닫기</p>
      {:else if hits.length === 0}
        <p class="px-4 py-3 text-xs text-fg-muted">결과 없음.</p>
      {:else}
        <ul class="max-h-[420px] overflow-y-auto divide-y divide-border">
          {#each hits as h, i}
            <li>
              <button
                type="button"
                onclick={() => activate(h)}
                onmouseenter={() => { activeIndex = i; }}
                class="w-full text-left px-4 py-2.5 transition-colors
                  {i === activeIndex ? 'bg-surface-2' : 'hover:bg-surface-2'}"
              >
                <div class="flex items-center gap-2">
                  <span class="text-xs text-fg-muted shrink-0">{kindIcon(h.kind)}</span>
                  <span class="text-sm truncate">{h.title}</span>
                  <span class="text-[10px] px-1 py-0.5 rounded bg-surface-3 text-fg-muted shrink-0">
                    {h.project_name}
                  </span>
                  <span class="ml-auto text-xs text-fg-muted shrink-0 tabular-nums">{h.score.toFixed(2)}</span>
                </div>
                {#if h.snippet}
                  <p class="text-xs text-fg-muted line-clamp-1 mt-0.5">{h.snippet}</p>
                {/if}
              </button>
            </li>
          {/each}
        </ul>
        <div class="px-4 py-2 text-[11px] text-fg-muted border-t border-border flex items-center justify-between">
          <span>↑↓ 이동 · ⏎ 열기 · ⌘⏎ 전체 결과</span>
          <button
            type="button"
            onclick={viewAllResults}
            class="text-accent hover:underline"
          >
            전체 결과 보기 →
          </button>
        </div>
      {/if}
    </div>
  </div>
{/if}
