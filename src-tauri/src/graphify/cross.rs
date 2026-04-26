//! 크로스 프로젝트 그래프 머지.
//!
//! 여러 프로젝트의 GraphifyGraph 를 받아 단일 CrossProjectGraph 로 합친다.
//! norm_label 이 같은 노드 간에 가상 브리지 엣지를 추가해 "두 프로젝트에서 같은 개념" 을 표현.

use std::collections::HashMap;

use serde::Serialize;

use crate::graphify::reader::{GraphifyConfidence, GraphifyGraph};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CrossProjectMember {
    pub project_id: String,
    pub project_name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CrossProjectNode {
    pub id: String,
    pub label: String,
    pub original_id: String,
    pub project_id: String,
    pub community: Option<i64>,
    pub file_type: Option<String>,
    pub source_file: Option<String>,
    pub norm_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CrossProjectEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub confidence: GraphifyConfidence,
    pub confidence_score: f64,
    pub project_id: Option<String>,
    pub is_bridge: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CrossProjectGraph {
    pub nodes: Vec<CrossProjectNode>,
    pub edges: Vec<CrossProjectEdge>,
    pub members: Vec<CrossProjectMember>,
}

const BRIDGE_RELATION: &str = "cross_project_alias";
const BRIDGE_SCORE: f64 = 0.5;

/// 노드 id 네임스페이싱: `{project_id}::{node_id}`.
fn ns_id(project_id: &str, node_id: &str) -> String {
    format!("{}::{}", project_id, node_id)
}

pub fn merge_graphs(
    members: &[CrossProjectMember],
    graphs: Vec<(String, GraphifyGraph)>,
    merge_labels: bool,
) -> CrossProjectGraph {
    let mut nodes: Vec<CrossProjectNode> = Vec::new();
    let mut edges: Vec<CrossProjectEdge> = Vec::new();

    // norm_label → [(project_id, ns_id)] 매핑 (브리지 후보)
    let mut by_label: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for (project_id, graph) in graphs {
        for n in graph.nodes {
            let ns = ns_id(&project_id, &n.id);
            if let Some(label) = n.norm_label.as_ref() {
                by_label
                    .entry(label.clone())
                    .or_default()
                    .push((project_id.clone(), ns.clone()));
            }
            nodes.push(CrossProjectNode {
                id: ns,
                label: n.label,
                original_id: n.id,
                project_id: project_id.clone(),
                community: n.community,
                file_type: n.file_type,
                source_file: n.source_file,
                norm_label: n.norm_label,
            });
        }
        for e in graph.edges {
            edges.push(CrossProjectEdge {
                source: ns_id(&project_id, &e.source),
                target: ns_id(&project_id, &e.target),
                relation: e.relation,
                confidence: e.confidence,
                confidence_score: e.confidence_score,
                project_id: Some(project_id.clone()),
                is_bridge: false,
            });
        }
    }

    if merge_labels {
        for entries in by_label.values() {
            // 동일 norm_label 의 노드들. 다른 프로젝트끼리만 페어 생성.
            for i in 0..entries.len() {
                for j in (i + 1)..entries.len() {
                    let (pa, na) = &entries[i];
                    let (pb, nb) = &entries[j];
                    if pa == pb {
                        continue;
                    }
                    edges.push(CrossProjectEdge {
                        source: na.clone(),
                        target: nb.clone(),
                        relation: BRIDGE_RELATION.to_string(),
                        confidence: GraphifyConfidence::Inferred,
                        confidence_score: BRIDGE_SCORE,
                        project_id: None,
                        is_bridge: true,
                    });
                }
            }
        }
    }

    CrossProjectGraph {
        nodes,
        edges,
        members: members.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphify::reader::{GraphifyEdge, GraphifyNode};

    fn member(id: &str, name: &str) -> CrossProjectMember {
        CrossProjectMember {
            project_id: id.to_string(),
            project_name: name.to_string(),
        }
    }

    fn node(id: &str, label: &str, norm: Option<&str>) -> GraphifyNode {
        GraphifyNode {
            id: id.to_string(),
            label: label.to_string(),
            file_type: None,
            source_file: None,
            source_location: None,
            community: None,
            norm_label: norm.map(String::from),
        }
    }

    fn edge(src: &str, tgt: &str) -> GraphifyEdge {
        GraphifyEdge {
            source: src.to_string(),
            target: tgt.to_string(),
            relation: "calls".to_string(),
            confidence: GraphifyConfidence::Extracted,
            confidence_score: 1.0,
            source_file: None,
            weight: 1.0,
        }
    }

    fn graph(nodes: Vec<GraphifyNode>, edges: Vec<GraphifyEdge>) -> GraphifyGraph {
        GraphifyGraph { nodes, edges, hyperedges: vec![] }
    }

    // BC #1: 빈 입력
    #[test]
    fn merge_empty_inputs() {
        let g = merge_graphs(&[], vec![], false);
        assert!(g.nodes.is_empty());
        assert!(g.edges.is_empty());
        assert!(g.members.is_empty());
    }

    // BC #2: 단일 프로젝트
    #[test]
    fn merge_single_project_namespaces_ids() {
        let members = vec![member("p1", "Project 1")];
        let graphs = vec![(
            "p1".to_string(),
            graph(vec![node("a", "A", None), node("b", "B", None)], vec![edge("a", "b")]),
        )];
        let cg = merge_graphs(&members, graphs, false);
        assert_eq!(cg.nodes.len(), 2);
        assert_eq!(cg.nodes[0].id, "p1::a");
        assert_eq!(cg.nodes[0].original_id, "a");
        assert_eq!(cg.nodes[0].project_id, "p1");
        assert_eq!(cg.edges.len(), 1);
        assert_eq!(cg.edges[0].source, "p1::a");
        assert_eq!(cg.edges[0].target, "p1::b");
        assert!(!cg.edges[0].is_bridge);
        assert_eq!(cg.edges[0].project_id.as_deref(), Some("p1"));
    }

    // BC #3: 두 프로젝트, merge_labels=true, 동일 norm_label
    #[test]
    fn merge_creates_bridge_for_matching_norm_label() {
        let members = vec![member("p1", "P1"), member("p2", "P2")];
        let graphs = vec![
            (
                "p1".to_string(),
                graph(vec![node("a", "JWT Validator", Some("jwt validator"))], vec![]),
            ),
            (
                "p2".to_string(),
                graph(vec![node("x", "JWT 검증", Some("jwt validator"))], vec![]),
            ),
        ];
        let cg = merge_graphs(&members, graphs, true);
        let bridges: Vec<_> = cg.edges.iter().filter(|e| e.is_bridge).collect();
        assert_eq!(bridges.len(), 1);
        let b = bridges[0];
        assert!(b.project_id.is_none());
        assert_eq!(b.relation, BRIDGE_RELATION);
        assert_eq!(b.confidence, GraphifyConfidence::Inferred);
        assert_eq!(b.confidence_score, BRIDGE_SCORE);
        let endpoints = (b.source.as_str(), b.target.as_str());
        assert!(endpoints == ("p1::a", "p2::x") || endpoints == ("p2::x", "p1::a"));
    }

    // BC #4: merge_labels=false 면 브리지 없음
    #[test]
    fn merge_no_bridge_when_disabled() {
        let members = vec![member("p1", "P1"), member("p2", "P2")];
        let graphs = vec![
            ("p1".to_string(), graph(vec![node("a", "X", Some("same"))], vec![])),
            ("p2".to_string(), graph(vec![node("b", "X", Some("same"))], vec![])),
        ];
        let cg = merge_graphs(&members, graphs, false);
        assert!(cg.edges.iter().all(|e| !e.is_bridge));
    }

    // BC #5: norm_label=None 노드는 브리지 후보 제외
    #[test]
    fn merge_skips_none_norm_label() {
        let members = vec![member("p1", "P1"), member("p2", "P2")];
        let graphs = vec![
            ("p1".to_string(), graph(vec![node("a", "X", None)], vec![])),
            ("p2".to_string(), graph(vec![node("b", "X", None)], vec![])),
        ];
        let cg = merge_graphs(&members, graphs, true);
        assert!(cg.edges.iter().all(|e| !e.is_bridge));
    }

    // BC #6: 같은 프로젝트 내 동일 norm_label 은 브리지 X
    #[test]
    fn merge_no_bridge_within_same_project() {
        let members = vec![member("p1", "P1")];
        let graphs = vec![(
            "p1".to_string(),
            graph(
                vec![
                    node("a", "JWT", Some("jwt")),
                    node("b", "JWT util", Some("jwt")),
                ],
                vec![],
            ),
        )];
        let cg = merge_graphs(&members, graphs, true);
        assert!(cg.edges.iter().all(|e| !e.is_bridge));
    }

    // 추가: 3 프로젝트 모두 동일 norm_label 이면 브리지가 3개 (3C2)
    #[test]
    fn merge_bridge_pairs_n_choose_2() {
        let members = vec![member("p1", "P1"), member("p2", "P2"), member("p3", "P3")];
        let graphs = vec![
            ("p1".to_string(), graph(vec![node("n", "X", Some("same"))], vec![])),
            ("p2".to_string(), graph(vec![node("n", "X", Some("same"))], vec![])),
            ("p3".to_string(), graph(vec![node("n", "X", Some("same"))], vec![])),
        ];
        let cg = merge_graphs(&members, graphs, true);
        let bridges: Vec<_> = cg.edges.iter().filter(|e| e.is_bridge).collect();
        assert_eq!(bridges.len(), 3);
    }
}
