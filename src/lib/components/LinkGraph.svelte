<script lang="ts">
  import {
    forceSimulation,
    forceLink,
    forceManyBody,
    forceCenter,
    forceCollide,
    type Simulation,
  } from "d3-force";
  import { zoom, zoomIdentity, type ZoomTransform } from "d3-zoom";
  import { select } from "d3-selection";
  import { drag } from "d3-drag";
  import type { GraphifyEdge, GraphifyGraph, GraphifyNode } from "$lib/types";
  import { onMount } from "svelte";

  interface Props {
    graph: GraphifyGraph;
    activeNodeIds?: Set<string> | null;
    onSelect?: (node: GraphifyNode) => void;
  }

  let { graph, activeNodeIds = null, onSelect }: Props = $props();

  // 12색 커뮤니티 팔레트 (Tableau 10 + 2 보충)
  const COMMUNITY_PALETTE: string[] = [
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2",
    "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    "#9c755f", "#bab0ac", "#86bc86", "#fb8072",
  ];
  const UNKNOWN_COLOR = "#888888";

  function nodeColor(community: number | null | undefined): string {
    if (community === null || community === undefined) return UNKNOWN_COLOR;
    return COMMUNITY_PALETTE[Math.abs(community) % COMMUNITY_PALETTE.length];
  }

  function nodeRadius(degree: number): number {
    return Math.sqrt(degree + 1) * 3 + 4;
  }

  // SimNode: d3-force가 직접 변형하므로 plain mutable 객체로 둠 (Svelte $state 우회).
  interface SimNode {
    id: string;
    label: string;
    community: number | null;
    source_file: string | null;
    raw: GraphifyNode;
    degree: number;
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
    edge: GraphifyEdge;
  }

  let canvasEl: HTMLCanvasElement;
  let containerEl: HTMLDivElement;
  let hoverEl = $state<{ x: number; y: number; label: string } | null>(null);

  let simNodes: SimNode[] = [];
  let simLinks: SimLink[] = [];
  let sim: Simulation<SimNode, SimLink> | null = null;
  let transform: ZoomTransform = zoomIdentity;
  let dpr = 1;
  let cssWidth = 800;
  let cssHeight = 600;
  let rafId: number | null = null;
  let dirty = true; // redraw 필요 플래그
  // 활성 필터 변화 감지를 위한 마지막 set (참조 비교)
  let lastActive: Set<string> | null = null;

  function requestRedraw(): void {
    dirty = true;
  }

  function ensureCanvasSize(): void {
    if (!canvasEl || !containerEl) return;
    const rect = containerEl.getBoundingClientRect();
    cssWidth = rect.width;
    cssHeight = rect.height;
    dpr = window.devicePixelRatio || 1;
    canvasEl.width = Math.round(cssWidth * dpr);
    canvasEl.height = Math.round(cssHeight * dpr);
    canvasEl.style.width = `${cssWidth}px`;
    canvasEl.style.height = `${cssHeight}px`;
    requestRedraw();
  }

  function nodeOpacity(id: string): number {
    if (!lastActive) return 1;
    return lastActive.has(id) ? 1 : 0.15;
  }

  function edgeOpacity(srcId: string, tgtId: string, score: number): number {
    const base = Math.max(0.3, Math.min(score, 1.0));
    if (!lastActive) return base;
    return lastActive.has(srcId) && lastActive.has(tgtId) ? base : 0.06;
  }

  function edgeStroke(confidence: string): { width: number; dash: number[] } {
    switch (confidence) {
      case "EXTRACTED": return { width: 1.0, dash: [] };
      case "INFERRED": return { width: 0.6, dash: [] };
      case "AMBIGUOUS": return { width: 0.6, dash: [3, 3] };
      default: return { width: 0.4, dash: [3, 3] };
    }
  }

  function draw(): void {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;

    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssWidth, cssHeight);
    // d3-zoom transform
    ctx.translate(transform.x, transform.y);
    ctx.scale(transform.k, transform.k);

    // Edges
    for (const l of simLinks) {
      const s = typeof l.source === "object" ? l.source : null;
      const t = typeof l.target === "object" ? l.target : null;
      if (!s || !t) continue;
      const stroke = edgeStroke(l.edge.confidence);
      ctx.beginPath();
      ctx.moveTo(s.x, s.y);
      ctx.lineTo(t.x, t.y);
      ctx.lineWidth = stroke.width / transform.k;
      ctx.setLineDash(stroke.dash.map((d) => d / transform.k));
      ctx.strokeStyle = `rgba(153,153,153,${edgeOpacity(s.id, t.id, l.edge.confidence_score)})`;
      ctx.stroke();
    }
    ctx.setLineDash([]);

    // Nodes
    for (const n of simNodes) {
      const r = nodeRadius(n.degree);
      const op = nodeOpacity(n.id);
      const color = nodeColor(n.community);
      ctx.beginPath();
      ctx.arc(n.x, n.y, r, 0, Math.PI * 2);
      ctx.fillStyle = applyAlpha(color, 0.85 * op);
      ctx.fill();
      ctx.lineWidth = 1.5 / transform.k;
      ctx.strokeStyle = applyAlpha(color, 0.4 * op);
      ctx.stroke();
    }

    // Labels (줌 충분히 들어왔을 때만 + viewport culling)
    if (transform.k > 0.8) {
      const fontPx = Math.min(11, 11 / transform.k);
      ctx.font = `${fontPx}px ui-sans-serif, system-ui, sans-serif`;
      ctx.textAlign = "center";
      ctx.fillStyle = "#cccccc";
      // viewport 안의 노드만 텍스트 그림
      const invK = 1 / transform.k;
      const minX = -transform.x * invK;
      const minY = -transform.y * invK;
      const maxX = (cssWidth - transform.x) * invK;
      const maxY = (cssHeight - transform.y) * invK;
      for (const n of simNodes) {
        if (n.x < minX || n.x > maxX || n.y < minY || n.y > maxY) continue;
        const r = nodeRadius(n.degree);
        ctx.fillText(n.label, n.x, n.y - r - 3);
      }
    }

    ctx.restore();
  }

  function applyAlpha(hex: string, alpha: number): string {
    // hex (#rrggbb) → rgba()
    const h = hex.replace("#", "");
    const r = parseInt(h.slice(0, 2), 16);
    const g = parseInt(h.slice(2, 4), 16);
    const b = parseInt(h.slice(4, 6), 16);
    return `rgba(${r},${g},${b},${alpha})`;
  }

  function loop(): void {
    if (sim && (sim.alpha() > sim.alphaMin() || dirty)) {
      draw();
      dirty = false;
    }
    rafId = requestAnimationFrame(loop);
  }

  function findNode(cssX: number, cssY: number): SimNode | null {
    // CSS 좌표 → 그래프 좌표
    const x = (cssX - transform.x) / transform.k;
    const y = (cssY - transform.y) / transform.k;
    let best: SimNode | null = null;
    let bestDist = Infinity;
    for (const n of simNodes) {
      const dx = n.x - x;
      const dy = n.y - y;
      const r = nodeRadius(n.degree);
      const d2 = dx * dx + dy * dy;
      if (d2 <= r * r && d2 < bestDist) {
        bestDist = d2;
        best = n;
      }
    }
    return best;
  }

  let dragNode: SimNode | null = null;
  let didDrag = false;

  function onPointerDown(ev: PointerEvent): void {
    const rect = canvasEl.getBoundingClientRect();
    const cx = ev.clientX - rect.left;
    const cy = ev.clientY - rect.top;
    const hit = findNode(cx, cy);
    if (hit) {
      dragNode = hit;
      didDrag = false;
      hit.fx = hit.x;
      hit.fy = hit.y;
      sim?.alphaTarget(0.3).restart();
      canvasEl.setPointerCapture(ev.pointerId);
      ev.stopPropagation();
    }
  }

  function onPointerMove(ev: PointerEvent): void {
    const rect = canvasEl.getBoundingClientRect();
    const cx = ev.clientX - rect.left;
    const cy = ev.clientY - rect.top;
    if (dragNode) {
      didDrag = true;
      const x = (cx - transform.x) / transform.k;
      const y = (cy - transform.y) / transform.k;
      dragNode.fx = x;
      dragNode.fy = y;
      requestRedraw();
      return;
    }
    // hover label
    const hit = findNode(cx, cy);
    if (hit) {
      hoverEl = { x: cx, y: cy, label: hit.label };
    } else if (hoverEl) {
      hoverEl = null;
    }
  }

  function onPointerUp(ev: PointerEvent): void {
    if (dragNode) {
      const node = dragNode;
      dragNode = null;
      sim?.alphaTarget(0);
      node.fx = null;
      node.fy = null;
      try {
        canvasEl.releasePointerCapture(ev.pointerId);
      } catch {
        // ignore
      }
      if (!didDrag) {
        onSelect?.(node.raw);
      }
    }
  }

  function rebuild(): void {
    sim?.stop();
    // degree 계산
    const degMap = new Map<string, number>();
    for (const e of graph.edges) {
      degMap.set(e.source, (degMap.get(e.source) ?? 0) + 1);
      degMap.set(e.target, (degMap.get(e.target) ?? 0) + 1);
    }

    simNodes = graph.nodes.map((n) => ({
      id: n.id,
      label: n.label,
      community: n.community,
      source_file: n.source_file,
      raw: n,
      degree: degMap.get(n.id) ?? 0,
      x: cssWidth / 2 + (Math.random() - 0.5) * 200,
      y: cssHeight / 2 + (Math.random() - 0.5) * 200,
      vx: 0,
      vy: 0,
      fx: null,
      fy: null,
    }));
    const idSet = new Set(simNodes.map((n) => n.id));
    simLinks = graph.edges
      .filter((e) => idSet.has(e.source) && idSet.has(e.target))
      .map((e) => ({ source: e.source, target: e.target, edge: e }));

    // 큰 그래프는 alpha decay 가속
    const decay = simNodes.length > 800 ? 0.05 : 0.0228;

    sim = forceSimulation(simNodes)
      .alphaDecay(decay)
      .force(
        "link",
        forceLink<SimNode, SimLink>(simLinks)
          .id((d) => d.id)
          .distance(60),
      )
      .force("charge", forceManyBody().strength(-120))
      .force("center", forceCenter(cssWidth / 2, cssHeight / 2))
      .force("collision", forceCollide<SimNode>().radius((d) => nodeRadius(d.degree) + 2));
    sim.on("tick", requestRedraw);
    requestRedraw();
  }

  onMount(() => {
    ensureCanvasSize();
    rebuild();

    const ro = new ResizeObserver(() => ensureCanvasSize());
    ro.observe(containerEl);

    // d3-zoom
    const z = zoom<HTMLCanvasElement, unknown>()
      .scaleExtent([0.1, 4])
      .filter((event) => {
        // 노드 위에서는 zoom/pan 비활성 (드래그 우선)
        if (event.type === "mousedown" || event.type === "pointerdown") {
          const rect = canvasEl.getBoundingClientRect();
          const cx = (event as PointerEvent).clientX - rect.left;
          const cy = (event as PointerEvent).clientY - rect.top;
          if (findNode(cx, cy)) return false;
        }
        return !(event as Event & { ctrlKey?: boolean }).ctrlKey;
      })
      .on("zoom", (event) => {
        transform = event.transform;
        requestRedraw();
      });
    select(canvasEl).call(z);

    rafId = requestAnimationFrame(loop);

    return () => {
      ro.disconnect();
      if (rafId !== null) cancelAnimationFrame(rafId);
      sim?.stop();
    };
  });

  // graph prop 변경 시 시뮬레이션 재구성
  $effect(() => {
    if (!canvasEl) return;
    // dependency: graph identity
    void graph;
    rebuild();
  });

  // activeNodeIds 변경 시 redraw 만 (simulation 영향 없음)
  $effect(() => {
    lastActive = activeNodeIds;
    requestRedraw();
  });
</script>

<div bind:this={containerEl} class="relative w-full h-full bg-surface-0 select-none">
  <canvas
    bind:this={canvasEl}
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointerleave={() => { hoverEl = null; }}
    class="block w-full h-full cursor-grab active:cursor-grabbing"
  ></canvas>
  {#if hoverEl}
    <div
      class="pointer-events-none absolute px-2 py-1 text-xs rounded bg-surface-2 border border-border text-fg shadow"
      style="left: {hoverEl.x + 12}px; top: {hoverEl.y + 12}px;"
    >
      {hoverEl.label}
    </div>
  {/if}
</div>
