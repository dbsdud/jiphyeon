import type { GraphNode, GraphEdge } from "./types";

export interface GraphFilter {
  query: string;
  typeFilter: string | null;
  tagFilter: string | null;
}

export const emptyFilter: GraphFilter = {
  query: "",
  typeFilter: null,
  tagFilter: null,
};

export function isFilterEmpty(filter: GraphFilter): boolean {
  return (
    filter.query.trim() === "" &&
    filter.typeFilter === null &&
    filter.tagFilter === null
  );
}

function matchesAnchor(node: GraphNode, filter: GraphFilter): boolean {
  const q = filter.query.trim().toLowerCase();
  if (q && !node.title.toLowerCase().includes(q)) return false;
  if (filter.typeFilter !== null && node.note_type !== filter.typeFilter) {
    return false;
  }
  if (filter.tagFilter !== null && !node.tags.includes(filter.tagFilter)) {
    return false;
  }
  return true;
}

/**
 * 앵커(매칭 노드) + 1-hop 이웃을 합친 활성 노드 ID 집합.
 *
 * - 필터가 모두 비어있으면 모든 노드 ID를 반환 (전체 활성)
 * - 매칭 앵커가 없으면 빈 Set (호출부가 전체 dim 처리)
 */
export function computeActiveIds(
  nodes: GraphNode[],
  edges: GraphEdge[],
  filter: GraphFilter,
): Set<string> {
  if (isFilterEmpty(filter)) {
    return new Set(nodes.map((n) => n.id));
  }

  const anchors = new Set<string>(
    nodes.filter((n) => matchesAnchor(n, filter)).map((n) => n.id),
  );
  if (anchors.size === 0) return new Set();

  const active = new Set<string>(anchors);
  for (const edge of edges) {
    if (anchors.has(edge.source) && !anchors.has(edge.target)) {
      active.add(edge.target);
    } else if (anchors.has(edge.target) && !anchors.has(edge.source)) {
      active.add(edge.source);
    }
  }
  return active;
}
