<script lang="ts">
  import type { ExplorerNode } from "$lib/types";
  import ExplorerTree from "./ExplorerTree.svelte";
  import { exploreStore } from "$lib/stores/theme.svelte";

  interface Props {
    nodes: ExplorerNode[];
    selectedPath: string;
    onSelect: (path: string) => void;
    depth?: number;
  }

  let { nodes, selectedPath, onSelect, depth = 0 }: Props = $props();

  function handleClick(node: ExplorerNode): void {
    if (node.kind === "folder") {
      exploreStore.toggle(node.path);
    } else {
      onSelect(node.path);
    }
  }
</script>

{#each nodes as node}
  {@const isExpanded = exploreStore.has(node.path)}
  {@const isFolder = node.kind === "folder"}
  {@const isSelected = !isFolder && selectedPath === node.path}
  <div>
    <div
      class="text-sm rounded flex items-center gap-1 transition-colors cursor-pointer
        {isSelected ? 'bg-accent/20 text-accent' : 'text-fg hover:bg-surface-2'}"
      style="padding: 0.2rem 0.5rem 0.2rem {depth * 0.75 + 0.5}rem"
      role="button"
      tabindex="0"
      onclick={() => handleClick(node)}
      onkeydown={(e) => {
        if (e.key === "Enter") handleClick(node);
      }}
    >
      {#if isFolder}
        <svg
          class="shrink-0 transition-transform {isExpanded ? 'rotate-90' : ''}"
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <polyline points="9 6 15 12 9 18" />
        </svg>
        <span class="text-xs">📁</span>
      {:else}
        <span class="w-[12px] shrink-0"></span>
        <span class="text-xs opacity-60">📄</span>
      {/if}
      <span class="truncate">{node.name}</span>
    </div>

    {#if isFolder && isExpanded && node.children.length > 0}
      <ExplorerTree
        nodes={node.children}
        {selectedPath}
        {onSelect}
        depth={depth + 1}
      />
    {/if}
  </div>
{/each}
