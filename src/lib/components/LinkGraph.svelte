<script lang="ts">
  import {
    forceSimulation,
    forceLink,
    forceManyBody,
    forceCenter,
    forceCollide,
  } from "d3-force";
  import { zoom, zoomIdentity } from "d3-zoom";
  import { select } from "d3-selection";
  import { drag } from "d3-drag";
  import type { LinkGraph, GraphNode, GraphEdge } from "$lib/types";

  interface Props {
    graph: LinkGraph;
  }

  let { graph }: Props = $props();

  let svgEl: SVGSVGElement;
  let width = $state(800);
  let height = $state(600);

  interface SimNode extends GraphNode {
    x: number;
    y: number;
    vx: number;
    vy: number;
    fx: number | null;
    fy: number | null;
  }

  interface SimLink {
    source: SimNode | string;
    target: SimNode | string;
  }

  let nodes = $state<SimNode[]>([]);
  let links = $state<SimLink[]>([]);
  let transform = $state({ x: 0, y: 0, k: 1 });

  const typeColors: Record<string, string> = {
    til: "#3b82f6",
    decision: "#eab308",
    reading: "#22c55e",
    meeting: "#a78bfa",
    idea: "#f59e0b",
    artifact: "#ec4899",
    clipping: "#06b6d4",
    moc: "#f97316",
    unknown: "#888888",
  };

  function nodeColor(type?: string): string {
    return typeColors[type ?? "unknown"] ?? typeColors.unknown;
  }

  function nodeRadius(linkCount: number): number {
    return Math.sqrt(linkCount + 1) * 3 + 4;
  }

  $effect(() => {
    if (!graph || !svgEl) return;

    const simNodes: SimNode[] = graph.nodes.map((n) => ({
      ...n,
      x: width / 2 + (Math.random() - 0.5) * 200,
      y: height / 2 + (Math.random() - 0.5) * 200,
      vx: 0,
      vy: 0,
      fx: null,
      fy: null,
    }));

    const simLinks: SimLink[] = graph.edges
      .filter((e) => simNodes.some((n) => n.id === e.source) && simNodes.some((n) => n.id === e.target))
      .map((e) => ({ source: e.source, target: e.target }));

    const sim = forceSimulation(simNodes)
      .force(
        "link",
        forceLink<SimNode, SimLink>(simLinks)
          .id((d) => d.id)
          .distance(60)
      )
      .force("charge", forceManyBody().strength(-120))
      .force("center", forceCenter(width / 2, height / 2))
      .force("collision", forceCollide<SimNode>().radius((d) => nodeRadius(d.link_count) + 2));

    sim.on("tick", () => {
      nodes = [...simNodes];
      links = [...simLinks];
    });

    // Zoom
    const zoomBehavior = zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.1, 4])
      .on("zoom", (event) => {
        transform = { x: event.transform.x, y: event.transform.y, k: event.transform.k };
      });

    select(svgEl).call(zoomBehavior);

    // Drag
    const dragBehavior = drag<SVGCircleElement, SimNode>()
      .on("start", (event, d) => {
        if (!event.active) sim.alphaTarget(0.3).restart();
        d.fx = d.x;
        d.fy = d.y;
      })
      .on("drag", (event, d) => {
        d.fx = event.x;
        d.fy = event.y;
      })
      .on("end", (event, d) => {
        if (!event.active) sim.alphaTarget(0);
        d.fx = null;
        d.fy = null;
      });

    // 시뮬레이션 안정화 후 드래그 적용
    const applyDrag = () => {
      select(svgEl)
        .selectAll<SVGCircleElement, SimNode>("circle.node")
        .data(simNodes, (d) => d.id)
        .call(dragBehavior);
    };

    sim.on("end", applyDrag);
    setTimeout(applyDrag, 100);

    return () => {
      sim.stop();
    };
  });
</script>

<svg
  bind:this={svgEl}
  class="w-full h-full bg-surface"
  viewBox="0 0 {width} {height}"
>
  <g transform="translate({transform.x},{transform.y}) scale({transform.k})">
    <!-- Edges -->
    {#each links as link}
      {@const s = typeof link.source === "object" ? link.source : null}
      {@const t = typeof link.target === "object" ? link.target : null}
      {#if s && t}
        <line
          x1={s.x}
          y1={s.y}
          x2={t.x}
          y2={t.y}
          stroke="#333"
          stroke-width="0.5"
          stroke-opacity="0.6"
        />
      {/if}
    {/each}

    <!-- Nodes -->
    {#each nodes as node}
      <a href="/view?path={encodeURIComponent(node.path)}">
        <circle
          class="node"
          cx={node.x}
          cy={node.y}
          r={nodeRadius(node.link_count)}
          fill={nodeColor(node.note_type)}
          fill-opacity="0.85"
          stroke={nodeColor(node.note_type)}
          stroke-width="1.5"
          stroke-opacity="0.4"
        />
        {#if transform.k > 0.6}
          <text
            x={node.x}
            y={node.y - nodeRadius(node.link_count) - 3}
            text-anchor="middle"
            fill="#ccc"
            font-size="{Math.min(10 / transform.k, 11)}"
            class="pointer-events-none"
          >
            {node.title}
          </text>
        {/if}
      </a>
    {/each}
  </g>
</svg>
