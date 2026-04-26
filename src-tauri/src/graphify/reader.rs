//! `graphify-out/graph.json` 파서.
//!
//! NetworkX `node_link_data` 변종 (키: `nodes`, `links`, `graph.hyperedges`).
//! v2.0 시각화 / 검색은 이 모듈이 변환한 도메인 모델을 사용한다.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const BOM: &[u8; 3] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, thiserror::Error)]
pub enum GraphifyError {
    #[error("graphify-out 디렉토리에 graph.json 이 없습니다. /graphify 를 먼저 실행하세요.")]
    NotRun,
    #[error("graph.json 읽기 실패: {0}")]
    Io(#[from] std::io::Error),
    #[error("graph.json 파싱 실패: {0}")]
    Parse(String),
}

impl serde::Serialize for GraphifyError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum GraphifyConfidence {
    Extracted,
    Inferred,
    Ambiguous,
    #[serde(other)]
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphifyNode {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub file_type: Option<String>,
    #[serde(default)]
    pub source_file: Option<String>,
    #[serde(default)]
    pub source_location: Option<String>,
    #[serde(default)]
    pub community: Option<i64>,
    #[serde(default)]
    pub norm_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphifyEdge {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub confidence: GraphifyConfidence,
    pub confidence_score: f64,
    #[serde(default)]
    pub source_file: Option<String>,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphifyHyperedge {
    pub id: String,
    pub label: String,
    pub nodes: Vec<String>,
    pub relation: String,
    pub confidence: GraphifyConfidence,
    pub confidence_score: f64,
    #[serde(default)]
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphifyGraph {
    pub nodes: Vec<GraphifyNode>,
    pub edges: Vec<GraphifyEdge>,
    pub hyperedges: Vec<GraphifyHyperedge>,
}

/// graphify-out 디렉토리에서 `graph.json` 을 읽어 도메인 모델로 변환.
/// 파일이 없으면 `GraphifyError::NotRun`.
pub fn read_graphify_graph(graphify_out_dir: &Path) -> Result<GraphifyGraph, GraphifyError> {
    let graph_json = graphify_out_dir.join("graph.json");
    if !graph_json.exists() {
        return Err(GraphifyError::NotRun);
    }
    let bytes = fs::read(&graph_json)?;
    let slice = if bytes.starts_with(BOM) { &bytes[3..] } else { &bytes[..] };
    let raw: RawGraphFile =
        serde_json::from_slice(slice).map_err(|e| GraphifyError::Parse(e.to_string()))?;
    Ok(raw.into_graph())
}

// --- raw deserialize ---

#[derive(Debug, Deserialize)]
struct RawGraphFile {
    #[serde(default)]
    nodes: Vec<RawNode>,
    #[serde(default)]
    links: Vec<RawLink>,
    #[serde(default)]
    graph: RawGraphMeta,
}

#[derive(Debug, Default, Deserialize)]
struct RawGraphMeta {
    #[serde(default)]
    hyperedges: Vec<RawHyperedge>,
}

#[derive(Debug, Deserialize)]
struct RawNode {
    id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    file_type: Option<String>,
    #[serde(default)]
    source_file: Option<String>,
    #[serde(default)]
    source_location: Option<String>,
    #[serde(default)]
    community: Option<i64>,
    #[serde(default)]
    norm_label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawLink {
    source: String,
    target: String,
    #[serde(default)]
    relation: Option<String>,
    #[serde(default)]
    confidence: Option<GraphifyConfidence>,
    #[serde(default)]
    confidence_score: Option<f64>,
    #[serde(default)]
    source_file: Option<String>,
    #[serde(default)]
    weight: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RawHyperedge {
    id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    nodes: Vec<String>,
    #[serde(default)]
    relation: Option<String>,
    #[serde(default)]
    confidence: Option<GraphifyConfidence>,
    #[serde(default)]
    confidence_score: Option<f64>,
    #[serde(default)]
    source_file: Option<String>,
}

impl RawGraphFile {
    fn into_graph(self) -> GraphifyGraph {
        let nodes = self
            .nodes
            .into_iter()
            .map(|n| GraphifyNode {
                label: n.label.clone().unwrap_or_else(|| n.id.clone()),
                id: n.id,
                file_type: n.file_type,
                source_file: n.source_file,
                source_location: n.source_location,
                community: n.community,
                norm_label: n.norm_label,
            })
            .collect();

        let edges = self
            .links
            .into_iter()
            .map(|l| GraphifyEdge {
                source: l.source,
                target: l.target,
                relation: l.relation.unwrap_or_default(),
                confidence: l.confidence.unwrap_or(GraphifyConfidence::Unknown),
                confidence_score: l.confidence_score.unwrap_or(1.0),
                source_file: l.source_file,
                weight: l.weight.unwrap_or(1.0),
            })
            .collect();

        let hyperedges = self
            .graph
            .hyperedges
            .into_iter()
            .map(|h| GraphifyHyperedge {
                label: h.label.clone().unwrap_or_else(|| h.id.clone()),
                id: h.id,
                nodes: h.nodes,
                relation: h.relation.unwrap_or_default(),
                confidence: h.confidence.unwrap_or(GraphifyConfidence::Unknown),
                confidence_score: h.confidence_score.unwrap_or(1.0),
                source_file: h.source_file,
            })
            .collect();

        GraphifyGraph { nodes, edges, hyperedges }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graphify")
    }

    fn copy_fixture(name: &str, target_dir: &Path) -> PathBuf {
        fs::create_dir_all(target_dir).unwrap();
        let dest = target_dir.join("graph.json");
        let src = fixture_dir().join(name);
        fs::copy(&src, &dest).unwrap();
        dest
    }

    // --- BC ---

    #[test]
    fn errors_when_dir_absent() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope");
        let r = read_graphify_graph(&missing);
        assert!(matches!(r, Err(GraphifyError::NotRun)));
    }

    #[test]
    fn errors_when_graph_json_absent() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        let r = read_graphify_graph(&out);
        assert!(matches!(r, Err(GraphifyError::NotRun)));
    }

    #[test]
    fn errors_on_invalid_json() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("graph.json"), b"{ not valid").unwrap();
        let r = read_graphify_graph(&out);
        assert!(matches!(r, Err(GraphifyError::Parse(_))));
    }

    #[test]
    fn parses_minimal_graph() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("minimal.json", &out);

        let g = read_graphify_graph(&out).unwrap();
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.nodes[0].id, "n_loner");
        assert!(g.edges.is_empty());
        assert!(g.hyperedges.is_empty());
    }

    #[test]
    fn parses_sample_graph_counts() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("sample.json", &out);

        let g = read_graphify_graph(&out).unwrap();
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.edges.len(), 3);
        assert_eq!(g.hyperedges.len(), 1);
        assert_eq!(g.hyperedges[0].nodes.len(), 3);
    }

    #[test]
    fn confidence_variants_parse_correctly() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("sample.json", &out);

        let g = read_graphify_graph(&out).unwrap();
        let confidences: Vec<_> = g.edges.iter().map(|e| e.confidence).collect();
        assert_eq!(
            confidences,
            vec![
                GraphifyConfidence::Extracted,
                GraphifyConfidence::Inferred,
                GraphifyConfidence::Unknown, // "WEIRD" -> Unknown
            ]
        );
    }

    #[test]
    fn missing_weight_and_score_default_to_one() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("sample.json", &out);

        let g = read_graphify_graph(&out).unwrap();
        // 마지막 엣지(WEIRD)는 weight + confidence_score 모두 누락
        let last = &g.edges[2];
        assert_eq!(last.weight, 1.0);
        assert_eq!(last.confidence_score, 1.0);
    }

    #[test]
    fn missing_node_metadata_is_none() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("sample.json", &out);

        let g = read_graphify_graph(&out).unwrap();
        let gamma = g.nodes.iter().find(|n| n.id == "n_c").unwrap();
        assert!(gamma.file_type.is_none());
        assert!(gamma.source_file.is_none());
        assert!(gamma.community.is_none());
        assert!(gamma.norm_label.is_none());
        // label 누락이면 id 폴백
        assert_eq!(gamma.label, "Gamma"); // 픽스처는 label이 있으므로 유지
    }

    #[test]
    fn strips_utf8_bom() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(
            br#"{"directed":false,"multigraph":false,"graph":{},"nodes":[],"links":[]}"#,
        );
        fs::write(out.join("graph.json"), &bytes).unwrap();

        let g = read_graphify_graph(&out).unwrap();
        assert!(g.nodes.is_empty());
    }
}
