<script lang="ts">
  import {
    forceSimulation,
    forceLink,
    forceManyBody,
    forceCenter,
    forceCollide,
  } from "d3-force";
  import { zoom } from "d3-zoom";
  import { select } from "d3-selection";
  import { drag } from "d3-drag";
  import type { GraphifyEdge, GraphifyGraph, GraphifyNode } from "$lib/types";

  interface Props {
    graph: GraphifyGraph;
    activeNodeIds?: Set<string> | null;
    onSelect?: (node: GraphifyNode) => void;
  }

  let { graph, activeNodeIds = null, onSelect }: Props = $props();

  // 12색 팔레트 (Tableau 10 + 2 보충). community % 12 매핑.
  const COMMUNITY_PALETTE: string[] = [
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2",
    "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    "#9c755f", "#bab0ac", "#86bc86", "#fb8072",
  ];
  const UNKNOWN_COLOR = "#888888";

  function nodeColor(community: number | null): string {
    if (community === null || community === undefined) return UNKNOWN_COLOR;
    const idx = Math.abs(community) % COMMUNITY_PALETTE.length;
    return COMMUNITY_PALETTE[idx];
  }

  function nodeOpacity(id: string): number {
    if (activeNodeIds === null) return 1;
    return activeNodeIds.has(id) ? 1 : 0.15;
  }

  function edgeOpacity(srcId: string, tgtId: string, score: number): number {
    const base = Math.max(0.3, Math.min(score, 1.0));
    if (activeNodeIds === null) return base;
    return activeNodeIds.has(srcId) && activeNodeIds.has(tgtId) ? base : 0.08;
  }

  function edgeStrokeWidth(confidence: string): number {
    switch (confidence) {
      case "EXTRACTED": return 1.0;
      case "INFERRED": return 0.6;
      case "AMBIGUOUS": return 0.6;
      default: return 0.4;
    }
  }

  function edgeDashArray(confidence: string): string {
    return confidence === "AMBIGUOUS" || confidence === "UNKNOWN" ? "3,3" : "";
  }

  let svgEl: SVGSVGElement;
  let width = $state(800);
  let height = $state(600);

  interface SimNode extends GraphifyNode {
    x: number;
    y: number;
    vx: number;
    vy: number;
    fx: number | null;
    fy: number | null;
    degree: number;
  }

  interface SimLink {
    source: SimNode | string;
    target: SimNode | string;
    edge: GraphifyEdge;
  }

  let nodes = $state<SimNode[]>([]);
  let links = $state<SimLink[]>([]);
  let transform = $state({ x: 0, y: 0, k: 1 });

  function nodeRadius(degree: number): number {
    return Math.sqrt(degree + 1) * 3 + 4;
  }

  function handleNodeClick(n: SimNode): void {
    onSelect?.(n);
  }

  $effect(() => {
    if (!graph || !svgEl) return;

    // degree 계산
    const degMap = new Map<string, number>();
    for (const e of graph.edges) {
      degMap.set(e.source, (degMap.get(e.source) ?? 0) + 1);
      degMap.set(e.target, (degMap.get(e.target) ?? 0) + 1);
    }

    const simNodes: SimNode[] = graph.nodes.map((n) => ({
      ...n,
      x: width / 2 + (Math.random() - 0.5) * 200,
      y: height / 2 + (Math.random() - 0.5) * 200,
      vx: 0,
      vy: 0,
      fx: null,
      fy: null,
      degree: degMap.get(n.id) ?? 0,
    }));
    const idSet = new Set(simNodes.map((n) => n.id));

    const simLinks: SimLink[] = graph.edges
      .filter((e) => idSet.has(e.source) && idSet.has(e.target))
      .map((e) => ({ source: e.source, target: e.target, edge: e }));

    const sim = forceSimulation(simNodes)
      .force(
        "link",
        forceLink<SimNode, SimLink>(simLinks)
          .id((d) => d.id)
          .distance(60),
      )
      .force("charge", forceManyBody().strength(-120))
      .force("center", forceCenter(width / 2, height / 2))
      .force("collision", forceCollide<SimNode>().radius((d) => nodeRadius(d.degree) + 2));

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

    const applyDrag = (): void => {
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
  class="w-full h-full bg-surface-0"
  viewBox="0 0 {width} {height}"
>
  <g transform="translate({transform.x},{transform.y}) scale({transform.k})">
    <!-- Edges -->
    {#each links as link}
      {@const s = typeof link.source === "object" ? link.source : null}
      {@const t = typeof link.target === "object" ? link.target : null}
      {#if s && t}
        <line
          class="edge"
          x1={s.x}
          y1={s.y}
          x2={t.x}
          y2={t.y}
          stroke="#999"
          stroke-width={edgeStrokeWidth(link.edge.confidence)}
          stroke-dasharray={edgeDashArray(link.edge.confidence)}
          stroke-opacity={edgeOpacity(s.id, t.id, link.edge.confidence_score)}
        />
      {/if}
    {/each}

    <!-- Nodes -->
    {#each nodes as node}
      {@const op = nodeOpacity(node.id)}
      {@const r = nodeRadius(node.degree)}
      {@const color = nodeColor(node.community)}
      <g
        class="cursor-pointer"
        onclick={() => handleNodeClick(node)}
        onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") handleNodeClick(node); }}
        role="button"
        tabindex="-1"
      >
        <circle
          class="node"
          cx={node.x}
          cy={node.y}
          r={r}
          fill={color}
          fill-opacity="0.85"
          stroke={color}
          stroke-width="1.5"
          stroke-opacity="0.4"
          opacity={op}
        />
        {#if transform.k > 0.6}
          <text
            x={node.x}
            y={node.y - r - 3}
            text-anchor="middle"
            fill="#ccc"
            font-size={Math.min(10 / transform.k, 11)}
            opacity={op}
            class="pointer-events-none select-none"
          >
            {node.label}
          </text>
        {/if}
      </g>
    {/each}
  </g>
</svg>

<style>
  circle.node,
  line.edge,
  text {
    transition: opacity 200ms ease, stroke-opacity 200ms ease;
  }
  @media (prefers-reduced-motion: reduce) {
    circle.node,
    line.edge,
    text {
      transition: none;
    }
  }
</style>
