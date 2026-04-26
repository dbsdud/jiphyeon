/**
 * GraphifyGraph (single) / CrossProjectGraph (cross) → LinkGraph 의 GraphView 어댑터.
 */

import type { GraphView } from "$lib/components/LinkGraph.svelte";
import type { CrossProjectGraph, GraphifyGraph } from "$lib/types";

export function adaptSingle(g: GraphifyGraph, projectId: string | null = null): GraphView {
  return {
    nodes: g.nodes.map((n) => ({
      id: n.id,
      label: n.label,
      community: n.community,
      project_id: projectId,
      source_file: n.source_file,
    })),
    edges: g.edges.map((e) => ({
      source: e.source,
      target: e.target,
      confidence: e.confidence,
      confidence_score: e.confidence_score,
      is_bridge: false,
    })),
  };
}

export function adaptCross(g: CrossProjectGraph): GraphView {
  return {
    nodes: g.nodes.map((n) => ({
      id: n.id,
      label: n.label,
      community: n.community,
      project_id: n.project_id,
      source_file: n.source_file,
    })),
    edges: g.edges.map((e) => ({
      source: e.source,
      target: e.target,
      confidence: e.confidence,
      confidence_score: e.confidence_score,
      is_bridge: e.is_bridge,
    })),
  };
}
