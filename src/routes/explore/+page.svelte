<script lang="ts">
  import { getNoteList, getTagList, getFolderTree, searchNotes, openInEditor } from "$lib/api";
  import type { NoteEntry, TagInfo, FolderNode, SearchResult } from "$lib/types";
  import FolderTree from "$lib/components/FolderTree.svelte";
  import { vaultRefresh } from "$lib/stores/vault.svelte";

  let notes = $state<NoteEntry[]>([]);
  let searchResults = $state<SearchResult[]>([]);
  let tags = $state<TagInfo[]>([]);
  let folders = $state<FolderNode[]>([]);
  let loading = $state(true);
  let isSearchMode = $state(false);

  // filters
  let filterFolder = $state<string | undefined>(undefined);
  let filterType = $state<string | undefined>(undefined);
  let filterStatus = $state<string | undefined>(undefined);
  let filterTag = $state<string | undefined>(undefined);
  let searchQuery = $state("");
  let sortBy = $state<string>("modified_at");

  async function loadSidebar() {
    [tags, folders] = await Promise.all([getTagList(), getFolderTree()]);
  }

  async function loadNotes() {
    loading = true;
    try {
      if (searchQuery.trim()) {
        isSearchMode = true;
        searchResults = await searchNotes(searchQuery.trim());
        notes = [];
      } else {
        isSearchMode = false;
        searchResults = [];
        notes = await getNoteList({
          folder: filterFolder,
          note_type: filterType,
          status: filterStatus,
          tag: filterTag,
          sort_by: sortBy === "modified_at" ? undefined : sortBy,
        });
      }
    } finally {
      loading = false;
    }
  }

  function highlightQuery(text: string, query: string): string {
    if (!query) return text;
    const escaped = query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
    return text.replace(new RegExp(`(${escaped})`, "gi"), "<mark class='bg-accent/30 text-fg rounded px-0.5'>$1</mark>");
  }

  function clearFilters() {
    filterFolder = undefined;
    filterType = undefined;
    filterStatus = undefined;
    filterTag = undefined;
    searchQuery = "";
    loadNotes();
  }

  function selectFolder(path: string) {
    filterFolder = filterFolder === path ? undefined : path;
    loadNotes();
  }

  function selectTag(name: string) {
    filterTag = filterTag === name ? undefined : name;
    loadNotes();
  }

  function selectType(type: string) {
    filterType = filterType === type ? undefined : type;
    loadNotes();
  }

  function selectStatus(status: string) {
    filterStatus = filterStatus === status ? undefined : status;
    loadNotes();
  }

  function onSearch() {
    loadNotes();
  }

  function formatDate(timestamp: number): string {
    if (!timestamp) return "";
    return new Date(timestamp * 1000).toLocaleDateString("ko-KR", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }

  function typeLabel(type: string): string {
    const labels: Record<string, string> = {
      til: "TIL", decision: "Decision", reading: "Reading", meeting: "Meeting",
      idea: "Idea", artifact: "Artifact", clipping: "Clipping", moc: "MOC", unknown: "Other",
    };
    return labels[type] ?? type;
  }

  const noteTypes = ["til", "decision", "reading", "meeting", "idea", "artifact", "clipping", "moc"];
  const statuses = ["seedling", "growing", "evergreen", "stale"];

  $effect(() => {
    vaultRefresh.version; // 볼트 변경 시 자동 재로드
    loadSidebar();
    loadNotes();
  });
</script>

<div class="flex h-full">
  <!-- Filter sidebar -->
  <aside class="w-48 border-r border-border p-3 overflow-y-auto shrink-0">
    <!-- Search -->
    <div class="mb-4">
      <input
        type="text"
        placeholder="Search..."
        class="w-full px-2 py-1.5 text-sm bg-surface-2 border border-border rounded text-fg placeholder:text-fg-muted focus:outline-none focus:border-accent"
        bind:value={searchQuery}
        onkeydown={(e) => { if (e.key === "Enter") onSearch(); }}
      />
    </div>

    <!-- Folders -->
    <div class="mb-4">
      <h4 class="text-xs font-medium text-fg-muted mb-2 uppercase tracking-wider">Folders</h4>
      <FolderTree nodes={folders} selectedPath={filterFolder} onSelect={selectFolder} />
    </div>

    <!-- Types -->
    <div class="mb-4">
      <h4 class="text-xs font-medium text-fg-muted mb-2 uppercase tracking-wider">Type</h4>
      {#each noteTypes as type}
        <button
          class="w-full text-left text-sm px-2 py-1 rounded transition-colors
            {filterType === type ? 'bg-accent/20 text-accent' : 'text-fg hover:bg-surface-2'}"
          onclick={() => selectType(type)}
        >
          {typeLabel(type)}
        </button>
      {/each}
    </div>

    <!-- Status -->
    <div class="mb-4">
      <h4 class="text-xs font-medium text-fg-muted mb-2 uppercase tracking-wider">Status</h4>
      {#each statuses as status}
        <button
          class="w-full text-left text-sm px-2 py-1 rounded transition-colors
            {filterStatus === status ? 'bg-accent/20 text-accent' : 'text-fg hover:bg-surface-2'}"
          onclick={() => selectStatus(status)}
        >
          {status}
        </button>
      {/each}
    </div>

    <!-- Tags -->
    <div class="mb-4">
      <h4 class="text-xs font-medium text-fg-muted mb-2 uppercase tracking-wider">Tags</h4>
      <div class="flex flex-wrap gap-1">
        {#each tags.slice(0, 20) as tag}
          <button
            class="text-xs px-1.5 py-0.5 rounded-full transition-colors
              {filterTag === tag.name ? 'bg-accent text-accent-fg' : 'bg-surface-3 text-fg hover:bg-surface-2'}"
            onclick={() => selectTag(tag.name)}
          >
            {tag.name}
          </button>
        {/each}
      </div>
    </div>

    {#if filterFolder || filterType || filterStatus || filterTag}
      <button
        class="w-full text-xs text-fg-muted hover:text-fg py-1"
        onclick={clearFilters}
      >
        Clear filters
      </button>
    {/if}
  </aside>

  <!-- Note list -->
  <div class="flex-1 overflow-y-auto">
    <div class="p-4">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-semibold">
          {isSearchMode ? "Search Results" : "Notes"}
          {#if !loading}
            <span class="text-sm text-fg-muted font-normal">({isSearchMode ? searchResults.length : notes.length})</span>
          {/if}
        </h2>
        <select
          class="text-sm bg-surface-2 border border-border rounded px-2 py-1 text-fg"
          bind:value={sortBy}
          onchange={() => loadNotes()}
        >
          <option value="modified_at">Recent</option>
          <option value="title">Title</option>
          <option value="size">Size</option>
        </select>
      </div>

      {#if loading}
        <p class="text-sm text-fg-muted">Loading...</p>
      {:else if isSearchMode}
        {#if searchResults.length === 0}
          <p class="text-sm text-fg-muted">No results found.</p>
        {:else}
          <div class="space-y-1">
            {#each searchResults as result}
              <a
                href="/view?path={encodeURIComponent(result.path)}"
                class="block px-3 py-2 rounded-lg hover:bg-surface-1 transition-colors group"
              >
                <div class="flex items-center justify-between">
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                      <span class="text-sm truncate">{result.title}</span>
                      <span class="text-[10px] px-1 py-0.5 rounded bg-surface-3 text-fg-muted shrink-0">{result.match_field}</span>
                      {#if result.frontmatter}
                        <span class="text-xs px-1.5 py-0.5 rounded bg-surface-2 text-fg-muted shrink-0">
                          {typeLabel(result.frontmatter.note_type)}
                        </span>
                      {/if}
                    </div>
                    <div class="text-xs text-fg-muted mt-0.5 truncate">{result.path}</div>
                  </div>
                  <div class="flex items-center gap-2 shrink-0 ml-3">
                    <span class="text-xs text-fg-muted">{formatDate(result.modified_at)}</span>
                    <button
                      class="text-fg-muted hover:text-accent opacity-0 group-hover:opacity-100 transition-opacity"
                      title="Open in Editor"
                      onclick={(e) => { e.preventDefault(); e.stopPropagation(); openInEditor(result.path); }}
                    >
                      <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 20 20" fill="currentColor">
                        <path d="M17.414 2.586a2 2 0 00-2.828 0L7 10.172V13h2.828l7.586-7.586a2 2 0 000-2.828z" />
                        <path fill-rule="evenodd" d="M2 6a2 2 0 012-2h4a1 1 0 010 2H4v10h10v-4a1 1 0 112 0v4a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" clip-rule="evenodd" />
                      </svg>
                    </button>
                  </div>
                </div>
                {#if result.snippet}
                  <p class="text-xs text-fg-muted mt-1 line-clamp-2">{@html highlightQuery(result.snippet, searchQuery)}</p>
                {/if}
              </a>
            {/each}
          </div>
        {/if}
      {:else if notes.length === 0}
        <p class="text-sm text-fg-muted">No notes found.</p>
      {:else}
        <div class="space-y-1">
          {#each notes as note}
            <a
              href="/view?path={encodeURIComponent(note.path)}"
              class="flex items-center justify-between px-3 py-2 rounded-lg hover:bg-surface-1 transition-colors group"
            >
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <span class="text-sm truncate">{note.title}</span>
                  {#if note.frontmatter}
                    <span class="text-xs px-1.5 py-0.5 rounded bg-surface-2 text-fg-muted shrink-0">
                      {typeLabel(note.frontmatter.note_type)}
                    </span>
                    {#if note.frontmatter.status}
                      <span class="text-xs text-fg-muted shrink-0">{note.frontmatter.status}</span>
                    {/if}
                  {/if}
                </div>
                <div class="text-xs text-fg-muted mt-0.5 truncate">{note.path}</div>
              </div>
              <div class="flex items-center gap-2 shrink-0 ml-3">
                <span class="text-xs text-fg-muted">{formatDate(note.modified_at)}</span>
                <button
                  class="text-fg-muted hover:text-accent opacity-0 group-hover:opacity-100 transition-opacity"
                  title="Open in Editor"
                  onclick={(e) => { e.preventDefault(); e.stopPropagation(); openInEditor(note.path); }}
                >
                  <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 20 20" fill="currentColor">
                    <path d="M17.414 2.586a2 2 0 00-2.828 0L7 10.172V13h2.828l7.586-7.586a2 2 0 000-2.828z" />
                    <path fill-rule="evenodd" d="M2 6a2 2 0 012-2h4a1 1 0 010 2H4v10h10v-4a1 1 0 112 0v4a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" clip-rule="evenodd" />
                  </svg>
                </button>
              </div>
            </a>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
