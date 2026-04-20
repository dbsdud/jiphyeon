<script lang="ts">
  import type { FolderNode } from "$lib/types";
  import FolderTree from "./FolderTree.svelte";
  import { exploreStore } from "$lib/stores/theme.svelte";

  interface Props {
    nodes: FolderNode[];
    selectedPath?: string;
    onSelect: (path: string) => void;
    depth?: number;
  }

  let { nodes, selectedPath, onSelect, depth = 0 }: Props = $props();

  function isSpecial(name: string): boolean {
    return name.startsWith("_");
  }

  function displayName(name: string): string {
    return name === "." ? "📓 볼트 루트" : name;
  }

  function handleRowClick(node: FolderNode): void {
    onSelect(node.path);
    if (node.children.length > 0) {
      exploreStore.toggle(node.path);
    }
  }
</script>

{#each nodes as node}
  {@const isExpanded = exploreStore.has(node.path)}
  {@const hasChildren = node.children.length > 0}
  <div>
    <div
      class="w-full text-left text-sm px-2 py-1 rounded flex items-center gap-1 transition-colors cursor-pointer
        {selectedPath === node.path ? 'bg-accent/20 text-accent' : 'text-fg hover:bg-surface-2'}
        {isSpecial(node.name) ? 'opacity-60' : ''}"
      style="padding-left: {depth * 0.75 + 0.5}rem"
      role="button"
      tabindex="0"
      onclick={() => handleRowClick(node)}
      onkeydown={(e) => {
        if (e.key === "Enter") handleRowClick(node);
      }}
    >
      {#if hasChildren}
        <svg
          class="shrink-0 transition-transform {isExpanded ? 'rotate-90' : ''}"
          width="14"
          height="14"
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
      {:else}
        <span class="w-[14px] shrink-0"></span>
      {/if}
      <span class="truncate">{displayName(node.name)}</span>
      <span class="text-fg-muted text-xs ml-auto shrink-0">{node.note_count}</span>
    </div>

    {#if hasChildren && isExpanded}
      <FolderTree
        nodes={node.children}
        {selectedPath}
        {onSelect}
        depth={depth + 1}
      />
    {/if}
  </div>
{/each}
