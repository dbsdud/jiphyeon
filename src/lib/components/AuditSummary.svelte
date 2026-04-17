<script lang="ts">
  import { getOrphanNotes, getNoteList } from "$lib/api";
  import type { NoteEntry } from "$lib/types";

  interface Props {
    brokenLinks: string[];
  }

  let { brokenLinks }: Props = $props();

  let orphanNotes = $state<NoteEntry[]>([]);
  let staleNotes = $state<NoteEntry[]>([]);
  let orphanExpanded = $state(false);
  let staleExpanded = $state(false);
  let brokenExpanded = $state(false);

  async function load() {
    [orphanNotes, staleNotes] = await Promise.all([
      getOrphanNotes(),
      getNoteList({ status: "stale" }),
    ]);
  }

  load();

  function formatDate(timestamp: number): string {
    if (!timestamp) return "";
    return new Date(timestamp * 1000).toLocaleDateString("ko-KR", {
      month: "short",
      day: "numeric",
    });
  }
</script>

<div class="space-y-3">
  <h3 class="text-sm font-medium text-muted">Audit Summary</h3>

  <!-- Orphan Notes -->
  <div class="bg-surface-1 rounded-lg border border-border overflow-hidden">
    <button
      class="w-full flex items-center justify-between px-4 py-2.5 hover:bg-surface-2 transition-colors text-left"
      onclick={() => { orphanExpanded = !orphanExpanded; }}
    >
      <div class="flex items-center gap-2">
        <span class="text-warning text-sm">Orphan Notes</span>
        <span class="text-xs px-1.5 py-0.5 rounded-full bg-warning/10 text-warning">
          {orphanNotes.length}
        </span>
      </div>
      <span class="text-xs text-muted">{orphanExpanded ? "▲" : "▼"}</span>
    </button>
    {#if orphanExpanded && orphanNotes.length > 0}
      <div class="border-t border-border divide-y divide-border max-h-48 overflow-y-auto">
        {#each orphanNotes as note}
          <a
            href="/view?path={encodeURIComponent(note.path)}"
            class="flex items-center justify-between px-4 py-2 hover:bg-surface-2 transition-colors text-sm"
          >
            <span class="truncate">{note.title}</span>
            <span class="text-xs text-muted shrink-0 ml-2">{formatDate(note.modified_at)}</span>
          </a>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Broken Links -->
  <div class="bg-surface-1 rounded-lg border border-border overflow-hidden">
    <button
      class="w-full flex items-center justify-between px-4 py-2.5 hover:bg-surface-2 transition-colors text-left"
      onclick={() => { brokenExpanded = !brokenExpanded; }}
    >
      <div class="flex items-center gap-2">
        <span class="text-danger text-sm">Broken Links</span>
        <span class="text-xs px-1.5 py-0.5 rounded-full bg-danger/10 text-danger">
          {brokenLinks.length}
        </span>
      </div>
      <span class="text-xs text-muted">{brokenExpanded ? "▲" : "▼"}</span>
    </button>
    {#if brokenExpanded && brokenLinks.length > 0}
      <div class="border-t border-border px-4 py-2 max-h-48 overflow-y-auto">
        <div class="flex flex-wrap gap-1.5">
          {#each brokenLinks as link}
            <span class="text-xs px-2 py-0.5 rounded-full bg-danger/10 text-danger border border-danger/20">
              [[{link}]]
            </span>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <!-- Stale Notes -->
  <div class="bg-surface-1 rounded-lg border border-border overflow-hidden">
    <button
      class="w-full flex items-center justify-between px-4 py-2.5 hover:bg-surface-2 transition-colors text-left"
      onclick={() => { staleExpanded = !staleExpanded; }}
    >
      <div class="flex items-center gap-2">
        <span class="text-muted text-sm">Stale Notes</span>
        <span class="text-xs px-1.5 py-0.5 rounded-full bg-surface-3 text-muted">
          {staleNotes.length}
        </span>
      </div>
      <span class="text-xs text-muted">{staleExpanded ? "▲" : "▼"}</span>
    </button>
    {#if staleExpanded && staleNotes.length > 0}
      <div class="border-t border-border divide-y divide-border max-h-48 overflow-y-auto">
        {#each staleNotes as note}
          <a
            href="/view?path={encodeURIComponent(note.path)}"
            class="flex items-center justify-between px-4 py-2 hover:bg-surface-2 transition-colors text-sm"
          >
            <span class="truncate">{note.title}</span>
            <span class="text-xs text-muted shrink-0 ml-2">{formatDate(note.modified_at)}</span>
          </a>
        {/each}
      </div>
    {/if}
  </div>
</div>
