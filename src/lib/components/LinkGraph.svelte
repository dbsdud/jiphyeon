<script lang="ts" module>
  /**
   * LinkGraph 가 직접 렌더하는 단일 노드/엣지 모델.
   * single 모드와 cross 모드의 어댑터가 GraphifyGraph / CrossProjectGraph 를 이 형태로 변환.
   */
  export interface NodeView {
    id: string;
    label: string;
    community: number | null;
    project_id: string | null;
    source_file: string | null;
  }

  export interface EdgeView {
    source: string;
    target: string;
    confidence: string;
    confidence_score: number;
    is_bridge: boolean;
  }

  export interface GraphView {
    nodes: NodeView[];
    edges: EdgeView[];
  }
</script>

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
  import { onMount } from "svelte";

  interface Props {
    graph: GraphView;
    activeNodeIds?: Set<string> | null;
    onSelect?: (nodeId: string) => void;
  }

  let { graph, activeNodeIds = null, onSelect }: Props = $props();

  // 12색 커뮤니티 팔레트
  const COMMUNITY_PALETTE: string[] = [
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2",
    "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    "#9c755f", "#bab0ac", "#86bc86", "#fb8072",
  ];
  const PROJECT_PALETTE: string[] = [
    "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728",
    "#9467bd", "#8c564b", "#e377c2", "#17becf",
  ];
  const UNKNOWN_COLOR = "#888888";
  const BRIDGE_COLOR = "#ff79c6";

  function communityColor(c: number | null): string {
    if (c === null || c === undefined) return UNKNOWN_COLOR;
    return COMMUNITY_PALETTE[Math.abs(c) % COMMUNITY_PALETTE.length];
  }

  function projectColor(pid: string | null): string {
    if (!pid) return UNKNOWN_COLOR;
    let h = 0;
    for (let i = 0; i < pid.length; i += 1) h = (h * 31 + pid.charCodeAt(i)) >>> 0;
    return PROJECT_PALETTE[h % PROJECT_PALETTE.length];
  }

  function nodeRadius(degree: number): number {
    return Math.sqrt(degree + 1) * 3 + 4;
  }

  interface SimNode {
    id: string;
    label: string;
    community: number | null;
    project_id: string | null;
    source_file: string | null;
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
    edge: EdgeView;
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
  let dirty = true;
  let lastActive: Set<string> | null = null;
  let hasMultipleProjects = false;

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

  interface EdgeStyle {
    color: string;
    width: number;
    dash: number[];
  }

  function edgeStyle(e: EdgeView): EdgeStyle {
    if (e.is_bridge) {
      return { color: BRIDGE_COLOR, width: 1.5, dash: [6, 3] };
    }
    switch (e.confidence) {
      case "EXTRACTED": return { color: "#999999", width: 1.0, dash: [] };
      case "INFERRED": return { color: "#999999", width: 0.6, dash: [] };
      case "AMBIGUOUS": return { color: "#999999", width: 0.6, dash: [3, 3] };
      default: return { color: "#999999", width: 0.4, dash: [3, 3] };
    }
  }

  function applyAlpha(hex: string, alpha: number): string {
    const h = hex.replace("#", "");
    const r = parseInt(h.slice(0, 2), 16);
    const g = parseInt(h.slice(2, 4), 16);
    const b = parseInt(h.slice(4, 6), 16);
    return `rgba(${r},${g},${b},${alpha})`;
  }

  function rgbaWithAlpha(rgbHex: string, alpha: number): string {
    return applyAlpha(rgbHex, alpha);
  }

  function draw(): void {
    if (!canvasEl) return;
    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;

    ctx.save();
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssWidth, cssHeight);
    ctx.translate(transform.x, transform.y);
    ctx.scale(transform.k, transform.k);

    // Edges
    for (const l of simLinks) {
      const s = typeof l.source === "object" ? l.source : null;
      const t = typeof l.target === "object" ? l.target : null;
      if (!s || !t) continue;
      const style = edgeStyle(l.edge);
      ctx.beginPath();
      ctx.moveTo(s.x, s.y);
      ctx.lineTo(t.x, t.y);
      ctx.lineWidth = style.width / transform.k;
      ctx.setLineDash(style.dash.map((d) => d / transform.k));
      ctx.strokeStyle = rgbaWithAlpha(style.color, edgeOpacity(s.id, t.id, l.edge.confidence_score));
      ctx.stroke();
    }
    ctx.setLineDash([]);

    // Nodes
    for (const n of simNodes) {
      const r = nodeRadius(n.degree);
      const op = nodeOpacity(n.id);
      const fill = communityColor(n.community);
      const stroke = hasMultipleProjects ? projectColor(n.project_id) : fill;
      ctx.beginPath();
      ctx.arc(n.x, n.y, r, 0, Math.PI * 2);
      ctx.fillStyle = applyAlpha(fill, 0.85 * op);
      ctx.fill();
      ctx.lineWidth = (hasMultipleProjects ? 2.0 : 1.5) / transform.k;
      ctx.strokeStyle = applyAlpha(stroke, (hasMultipleProjects ? 0.9 : 0.4) * op);
      ctx.stroke();
    }

    // Labels (viewport culling)
    if (transform.k > 0.8) {
      const fontPx = Math.min(11, 11 / transform.k);
      ctx.font = `${fontPx}px ui-sans-serif, system-ui, sans-serif`;
      ctx.textAlign = "center";
      ctx.fillStyle = "#cccccc";
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

  function loop(): void {
    if (sim && (sim.alpha() > sim.alphaMin() || dirty)) {
      draw();
      dirty = false;
    }
    rafId = requestAnimationFrame(loop);
  }

  function findNode(cssX: number, cssY: number): SimNode | null {
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
        onSelect?.(node.id);
      }
    }
  }

  function rebuild(): void {
    sim?.stop();
    const degMap = new Map<string, number>();
    for (const e of graph.edges) {
      degMap.set(e.source, (degMap.get(e.source) ?? 0) + 1);
      degMap.set(e.target, (degMap.get(e.target) ?? 0) + 1);
    }

    simNodes = graph.nodes.map((n) => ({
      id: n.id,
      label: n.label,
      community: n.community,
      project_id: n.project_id,
      source_file: n.source_file,
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

    const projectSet = new Set(simNodes.map((n) => n.project_id).filter((p) => p !== null));
    hasMultipleProjects = projectSet.size > 1;

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

    const z = zoom<HTMLCanvasElement, unknown>()
      .scaleExtent([0.1, 4])
      .filter((event) => {
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

  $effect(() => {
    if (!canvasEl) return;
    void graph;
    rebuild();
  });

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
