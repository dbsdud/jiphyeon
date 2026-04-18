<script lang="ts">
  import type { FolderNode } from "$lib/types";
  import FolderTree from "./FolderTree.svelte";

  interface Props {
    nodes: FolderNode[];
    selectedPath?: string;
    onSelect: (path: string) => void;
    depth?: number;
  }

  let { nodes, selectedPath, onSelect, depth = 0 }: Props = $props();

  let expanded = $state<Set<string>>(new Set());

  function toggle(path: string) {
    const next = new Set(expanded);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    expanded = next;
  }
</script>

{#each nodes as node}
  <div>
    <div
      class="w-full text-left text-sm px-2 py-1 rounded flex items-center gap-1 transition-colors cursor-pointer
        {selectedPath === node.path ? 'bg-accent/20 text-accent' : 'text-fg hover:bg-surface-2'}"
      style="padding-left: {depth * 0.75 + 0.5}rem"
      role="button"
      tabindex="0"
      onclick={() => onSelect(node.path)}
      onkeydown={(e) => { if (e.key === 'Enter') onSelect(node.path); }}
    >
      {#if node.children.length > 0}
        <button
          class="text-fg-muted hover:text-fg shrink-0 w-3 text-[10px]"
          onclick={(e) => { e.stopPropagation(); toggle(node.path); }}
        >
          {expanded.has(node.path) ? "▾" : "▸"}
        </button>
      {:else}
        <span class="w-3 shrink-0"></span>
      {/if}
      <span class="truncate">{node.name}</span>
      <span class="text-fg-muted text-xs ml-auto shrink-0">{node.note_count}</span>
    </div>

    {#if node.children.length > 0 && expanded.has(node.path)}
      <FolderTree
        nodes={node.children}
        {selectedPath}
        {onSelect}
        depth={depth + 1}
      />
    {/if}
  </div>
{/each}
