<script lang="ts">
  import { getProjectExplorerTree } from "$lib/api";
  import type { ExplorerNode } from "$lib/types";
  import ExplorerTree from "$lib/components/ExplorerTree.svelte";
  import NoteViewer from "$lib/components/NoteViewer.svelte";
  import { vaultRefresh } from "$lib/stores/vault.svelte";

  let tree = $state<ExplorerNode | null>(null);
  let selectedPath = $state<string>("");
  let error = $state("");

  async function loadTree(): Promise<void> {
    error = "";
    try {
      tree = await getProjectExplorerTree();
    } catch (e) {
      error = String(e);
      tree = null;
    }
  }

  function selectFile(path: string): void {
    selectedPath = path;
  }

  $effect(() => {
    vaultRefresh.version;
    selectedPath = "";
    loadTree();
  });
</script>

<div class="flex h-full">
  <!-- Tree (folders + files) -->
  <aside class="w-64 border-r border-border overflow-y-auto shrink-0 py-2">
    {#if tree}
      <!-- root 자체는 표시하지 않고 children 만 노출 -->
      <ExplorerTree
        nodes={tree.children}
        {selectedPath}
        onSelect={selectFile}
      />
      {#if tree.children.length === 0}
        <p class="px-3 py-2 text-xs text-fg-muted">docs/ 가 비어있습니다.</p>
      {/if}
    {:else if error}
      <p class="px-3 py-2 text-xs text-danger">{error}</p>
    {:else}
      <p class="px-3 py-2 text-xs text-fg-muted">Loading...</p>
    {/if}
  </aside>

  <!-- Viewer -->
  <div class="flex-1 overflow-y-auto">
    <NoteViewer notePath={selectedPath} />
  </div>
</div>
