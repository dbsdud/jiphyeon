<script lang="ts">
  import {
    listProjectFiles,
    getProjectFolderTree,
    openInEditor,
  } from "$lib/api";
  import type { FolderNode, ProjectFileEntry } from "$lib/types";
  import FolderTree from "$lib/components/FolderTree.svelte";
  import { vaultRefresh } from "$lib/stores/vault.svelte";

  let tree = $state<FolderNode | null>(null);
  let files = $state<ProjectFileEntry[]>([]);
  let selectedPath = $state<string>("");
  let loading = $state(true);
  let error = $state("");

  async function loadTree(): Promise<void> {
    try {
      tree = await getProjectFolderTree();
    } catch (e) {
      error = String(e);
    }
  }

  async function loadFiles(): Promise<void> {
    loading = true;
    error = "";
    try {
      files = await listProjectFiles(selectedPath || null);
    } catch (e) {
      error = String(e);
      files = [];
    } finally {
      loading = false;
    }
  }

  function selectFolder(path: string): void {
    selectedPath = path;
    loadFiles();
  }

  function formatDate(ts: number): string {
    if (!ts) return "";
    return new Date(ts * 1000).toLocaleDateString("ko-KR", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  }

  function typeLabel(type: string): string {
    const labels: Record<string, string> = {
      til: "TIL",
      decision: "Decision",
      reading: "Reading",
      meeting: "Meeting",
      idea: "Idea",
      artifact: "Artifact",
      clipping: "Clipping",
      moc: "MOC",
      unknown: "Other",
    };
    return labels[type] ?? type;
  }

  $effect(() => {
    vaultRefresh.version;
    selectedPath = "";
    loadTree();
    loadFiles();
  });
</script>

<div class="flex h-full">
  <!-- Folder tree -->
  <aside class="w-56 border-r border-border p-3 overflow-y-auto shrink-0">
    <h4 class="text-xs font-medium text-fg-muted mb-2 uppercase tracking-wider">📁 docs</h4>
    {#if tree}
      <FolderTree
        nodes={[tree]}
        selectedPath={selectedPath}
        onSelect={selectFolder}
      />
    {:else if error}
      <p class="text-xs text-danger">{error}</p>
    {:else}
      <p class="text-xs text-fg-muted">Loading...</p>
    {/if}
  </aside>

  <!-- File list -->
  <div class="flex-1 overflow-y-auto">
    <div class="p-4">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-semibold">
          {selectedPath || "docs"}
          {#if !loading}
            <span class="text-sm text-fg-muted font-normal">({files.length})</span>
          {/if}
        </h2>
      </div>

      {#if loading}
        <p class="text-sm text-fg-muted">Loading...</p>
      {:else if error}
        <p class="text-sm text-danger">{error}</p>
      {:else if files.length === 0}
        <p class="text-sm text-fg-muted">이 폴더에는 .md 파일이 없습니다.</p>
      {:else}
        <div class="space-y-1">
          {#each files as file}
            <a
              href="/view?path={encodeURIComponent(file.path)}"
              class="flex items-center justify-between px-3 py-2 rounded-lg hover:bg-surface-1 transition-colors group"
            >
              <div class="min-w-0 flex-1">
                <div class="flex items-center gap-2">
                  <span class="text-sm truncate">{file.title}</span>
                  {#if file.note_type}
                    <span class="text-xs px-1.5 py-0.5 rounded bg-surface-2 text-fg-muted shrink-0">
                      {typeLabel(file.note_type)}
                    </span>
                  {/if}
                </div>
                <div class="text-xs text-fg-muted mt-0.5 truncate">{file.path}</div>
              </div>
              <div class="flex items-center gap-2 shrink-0 ml-3">
                <span class="text-xs text-fg-muted">{formatDate(file.modified_at)}</span>
                <button
                  class="text-fg-muted hover:text-accent opacity-0 group-hover:opacity-100 transition-opacity"
                  title="Open in Editor"
                  onclick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    openInEditor(file.path);
                  }}
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
