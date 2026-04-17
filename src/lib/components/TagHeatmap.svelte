<script lang="ts">
  import { treemap, hierarchy, treemapSquarify } from "d3-hierarchy";
  import { scaleSequential } from "d3-scale";
  import type { TagInfo } from "$lib/types";

  interface Props {
    tags: TagInfo[];
    height?: number;
  }

  let { tags, height = 220 }: Props = $props();

  let containerEl = $state<HTMLDivElement | null>(null);
  let width = $state(300);

  $effect(() => {
    if (containerEl) {
      const observer = new ResizeObserver((entries) => {
        for (const entry of entries) {
          width = entry.contentRect.width;
        }
      });
      observer.observe(containerEl);
      width = containerEl.clientWidth;
      return () => observer.disconnect();
    }
  });

  interface TreemapRect {
    x0: number;
    y0: number;
    x1: number;
    y1: number;
    name: string;
    count: number;
  }

  let rects = $derived.by(() => {
    if (tags.length === 0) return [];

    const root = hierarchy({ children: tags.map((t) => ({ ...t, value: t.count })) })
      .sum((d: any) => d.value || 0);

    const layout = treemap<any>()
      .size([width, height])
      .padding(2)
      .tile(treemapSquarify);

    layout(root);

    return root.leaves().map((leaf: any) => ({
      x0: leaf.x0,
      y0: leaf.y0,
      x1: leaf.x1,
      y1: leaf.y1,
      name: leaf.data.name,
      count: leaf.data.count,
    })) as TreemapRect[];
  });

  let maxCount = $derived(Math.max(...tags.map((t) => t.count), 1));

  function fillColor(count: number): string {
    const t = count / maxCount;
    const r = Math.round(30 + t * 29);
    const g = Math.round(30 + t * 100);
    const b = Math.round(50 + t * 196);
    return `rgb(${r}, ${g}, ${b})`;
  }

  let hoveredTag = $state<string | null>(null);
</script>

<div bind:this={containerEl} class="w-full">
<svg {width} {height} class="rounded-lg w-full">
  {#each rects as rect}
    {@const w = rect.x1 - rect.x0}
    {@const h = rect.y1 - rect.y0}
    <a href="/explore?tag={encodeURIComponent(rect.name)}">
      <g
        role="img"
        aria-label={rect.name}
        onmouseenter={() => { hoveredTag = rect.name; }}
        onmouseleave={() => { hoveredTag = null; }}
      >
        <rect
          x={rect.x0}
          y={rect.y0}
          width={w}
          height={h}
          fill={fillColor(rect.count)}
          rx="3"
          class="transition-opacity"
          opacity={hoveredTag && hoveredTag !== rect.name ? 0.5 : 1}
        />
        {#if w > 30 && h > 16}
          <text
            x={rect.x0 + w / 2}
            y={rect.y0 + h / 2}
            text-anchor="middle"
            dominant-baseline="central"
            fill="white"
            font-size={Math.min(w / rect.name.length * 1.4, h * 0.5, 13)}
            class="pointer-events-none"
          >
            {rect.name}
          </text>
        {/if}
        {#if w > 20 && h > 30}
          <text
            x={rect.x0 + w / 2}
            y={rect.y0 + h / 2 + 10}
            text-anchor="middle"
            dominant-baseline="central"
            fill="rgba(255,255,255,0.5)"
            font-size="9"
            class="pointer-events-none"
          >
            {rect.count}
          </text>
        {/if}
      </g>
    </a>
  {/each}
</svg>

{#if hoveredTag}
  <div class="text-xs text-muted mt-1">
    {hoveredTag}: {tags.find((t) => t.name === hoveredTag)?.count ?? 0} notes
  </div>
{/if}
</div>
