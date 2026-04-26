//! `graphify-out/GRAPH_REPORT.md` 파서. 대시보드 카드용.
//!
//! 라인 단위 스캐너 + regex 로 섹션별 항목을 추출한다. graphify 가 보고서 포맷을
//! 점진적으로 바꿀 수 있어, 각 섹션은 누락 가능 (Option / 빈 vec).

use std::fs;
use std::path::Path;

use regex::Regex;
use serde::Serialize;

use crate::graphify::reader::{GraphifyConfidence, GraphifyError};

const BOM: &[u8; 3] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct GraphReportSummary {
    pub nodes_count: Option<usize>,
    pub edges_count: Option<usize>,
    pub communities_count: Option<usize>,
    pub extracted_pct: Option<f64>,
    pub inferred_pct: Option<f64>,
    pub ambiguous_pct: Option<f64>,
    pub token_input: Option<u64>,
    pub token_output: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphReportGodNode {
    pub rank: usize,
    pub name: String,
    pub edge_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphReportSurprisingConnection {
    pub source: String,
    pub target: String,
    pub relation: String,
    pub confidence: GraphifyConfidence,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GraphReportCommunity {
    pub id: i64,
    pub label: String,
    pub cohesion: Option<f64>,
    pub nodes_count: Option<usize>,
    pub sample_nodes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct GraphReport {
    pub generated_at: Option<String>,
    pub project_root: Option<String>,
    pub summary: GraphReportSummary,
    pub god_nodes: Vec<GraphReportGodNode>,
    pub surprising_connections: Vec<GraphReportSurprisingConnection>,
    pub communities: Vec<GraphReportCommunity>,
}

pub fn read_graphify_report(graphify_out_dir: &Path) -> Result<GraphReport, GraphifyError> {
    let report_path = graphify_out_dir.join("GRAPH_REPORT.md");
    if !report_path.exists() {
        return Err(GraphifyError::NotRun);
    }
    let bytes = fs::read(&report_path)?;
    let slice = if bytes.starts_with(BOM) { &bytes[3..] } else { &bytes[..] };
    let text = std::str::from_utf8(slice)
        .map_err(|e| GraphifyError::Parse(format!("UTF-8 decode 실패: {e}")))?;
    Ok(parse_report(text))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Summary,
    GodNodes,
    Surprising,
    Communities,
    Other,
}

fn parse_report(text: &str) -> GraphReport {
    let title_re = Regex::new(r"^# Graph Report - (.+?)\s*\((\d{4}-\d{2}-\d{2})\)\s*$").unwrap();
    let counts_re = Regex::new(
        r"(?P<nodes>\d+)\s+nodes\s+·\s+(?P<edges>\d+)\s+edges\s+·\s+(?P<communities>\d+)\s+communities",
    )
    .unwrap();
    let extraction_re = Regex::new(
        r"(?P<ext>\d+(?:\.\d+)?)%\s+EXTRACTED\s+·\s+(?P<inf>\d+(?:\.\d+)?)%\s+INFERRED\s+·\s+(?P<amb>\d+(?:\.\d+)?)%\s+AMBIGUOUS",
    )
    .unwrap();
    let token_re =
        Regex::new(r"Token cost:\s+(?P<input>\d+)\s+input\s+·\s+(?P<output>\d+)\s+output").unwrap();
    let god_re = Regex::new(r"^\s*(?P<rank>\d+)\.\s+`(?P<name>.+?)`\s+-\s+(?P<edges>\d+)\s+edges?\s*$").unwrap();
    let surprising_re = Regex::new(
        r"^\s*-\s+`(?P<src>.+?)`\s+--(?P<rel>.+?)-->\s+`(?P<tgt>.+?)`\s+\[(?P<conf>\w+)\]",
    )
    .unwrap();
    let community_head_re =
        Regex::new(r#"^###\s+Community\s+(?P<id>-?\d+)\s+-\s+"(?P<label>.+)"\s*$"#).unwrap();
    let cohesion_re = Regex::new(r"^Cohesion:\s+(?P<v>[0-9]+(?:\.[0-9]+)?)\s*$").unwrap();
    let nodes_line_re =
        Regex::new(r"^Nodes\s+\((?P<n>\d+)\):\s+(?P<list>.+?)(?:\s+\(\+\d+\s+more\))?\s*$")
            .unwrap();

    let mut report = GraphReport::default();
    let mut section = Section::None;
    let mut current_community: Option<GraphReportCommunity> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim_end();

        // 제목 라인
        if let Some(c) = title_re.captures(line) {
            report.project_root = Some(c[1].trim().to_string());
            report.generated_at = Some(c[2].to_string());
            continue;
        }

        // 섹션 전환
        if line.starts_with("## ") {
            // 진행 중이던 community 가 있으면 push
            if let Some(c) = current_community.take() {
                report.communities.push(c);
            }
            let head = line.trim_start_matches("## ").trim();
            section = if head.starts_with("Summary") {
                Section::Summary
            } else if head.starts_with("God Nodes") {
                Section::GodNodes
            } else if head.starts_with("Surprising Connections") {
                Section::Surprising
            } else if head == "Communities" {
                Section::Communities
            } else {
                Section::Other
            };
            continue;
        }

        match section {
            Section::Summary => {
                if let Some(c) = counts_re.captures(line) {
                    report.summary.nodes_count = c.name("nodes").and_then(|m| m.as_str().parse().ok());
                    report.summary.edges_count = c.name("edges").and_then(|m| m.as_str().parse().ok());
                    report.summary.communities_count =
                        c.name("communities").and_then(|m| m.as_str().parse().ok());
                }
                if let Some(c) = extraction_re.captures(line) {
                    report.summary.extracted_pct = c.name("ext").and_then(|m| m.as_str().parse().ok());
                    report.summary.inferred_pct = c.name("inf").and_then(|m| m.as_str().parse().ok());
                    report.summary.ambiguous_pct = c.name("amb").and_then(|m| m.as_str().parse().ok());
                }
                if let Some(c) = token_re.captures(line) {
                    report.summary.token_input = c.name("input").and_then(|m| m.as_str().parse().ok());
                    report.summary.token_output = c.name("output").and_then(|m| m.as_str().parse().ok());
                }
            }
            Section::GodNodes => {
                if let Some(c) = god_re.captures(line) {
                    report.god_nodes.push(GraphReportGodNode {
                        rank: c["rank"].parse().unwrap_or(0),
                        name: c["name"].to_string(),
                        edge_count: c["edges"].parse().unwrap_or(0),
                    });
                }
            }
            Section::Surprising => {
                if let Some(c) = surprising_re.captures(line) {
                    let conf = serde_json::from_str::<GraphifyConfidence>(&format!("\"{}\"", &c["conf"]))
                        .unwrap_or(GraphifyConfidence::Unknown);
                    report.surprising_connections.push(GraphReportSurprisingConnection {
                        source: c["src"].to_string(),
                        target: c["tgt"].to_string(),
                        relation: c["rel"].to_string(),
                        confidence: conf,
                    });
                }
            }
            Section::Communities => {
                if let Some(c) = community_head_re.captures(line) {
                    if let Some(prev) = current_community.take() {
                        report.communities.push(prev);
                    }
                    let id: i64 = c["id"].parse().unwrap_or(0);
                    current_community = Some(GraphReportCommunity {
                        id,
                        label: c["label"].to_string(),
                        cohesion: None,
                        nodes_count: None,
                        sample_nodes: Vec::new(),
                    });
                } else if let Some(c) = cohesion_re.captures(line) {
                    if let Some(comm) = current_community.as_mut() {
                        comm.cohesion = c.name("v").and_then(|m| m.as_str().parse().ok());
                    }
                } else if let Some(c) = nodes_line_re.captures(line) {
                    if let Some(comm) = current_community.as_mut() {
                        comm.nodes_count = c.name("n").and_then(|m| m.as_str().parse().ok());
                        comm.sample_nodes = c["list"]
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(c) = current_community.take() {
        report.communities.push(c);
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/graphify")
    }

    fn copy_fixture(name: &str, target_dir: &Path) {
        fs::create_dir_all(target_dir).unwrap();
        let src = fixture_dir().join(name);
        let dest = target_dir.join("GRAPH_REPORT.md");
        fs::copy(&src, &dest).unwrap();
    }

    #[test]
    fn errors_when_dir_absent() {
        let dir = TempDir::new().unwrap();
        let r = read_graphify_report(&dir.path().join("nope"));
        assert!(matches!(r, Err(GraphifyError::NotRun)));
    }

    #[test]
    fn errors_when_report_md_absent() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        let r = read_graphify_report(&out);
        assert!(matches!(r, Err(GraphifyError::NotRun)));
    }

    #[test]
    fn parses_minimal_report_title_only() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("report-minimal.md", &out);
        let r = read_graphify_report(&out).unwrap();
        assert_eq!(r.project_root.as_deref(), Some("/tmp/empty"));
        assert_eq!(r.generated_at.as_deref(), Some("2026-04-26"));
        assert!(r.god_nodes.is_empty());
        assert!(r.communities.is_empty());
        assert_eq!(r.summary.nodes_count, None);
    }

    #[test]
    fn parses_summary_counts() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("report-sample.md", &out);
        let r = read_graphify_report(&out).unwrap();
        assert_eq!(r.summary.nodes_count, Some(1166));
        assert_eq!(r.summary.edges_count, Some(1934));
        assert_eq!(r.summary.communities_count, Some(148));
    }

    #[test]
    fn parses_summary_extraction_percentages() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("report-sample.md", &out);
        let r = read_graphify_report(&out).unwrap();
        assert_eq!(r.summary.extracted_pct, Some(63.0));
        assert_eq!(r.summary.inferred_pct, Some(37.0));
        assert_eq!(r.summary.ambiguous_pct, Some(0.0));
    }

    #[test]
    fn parses_summary_tokens() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("report-sample.md", &out);
        let r = read_graphify_report(&out).unwrap();
        assert_eq!(r.summary.token_input, Some(12345));
        assert_eq!(r.summary.token_output, Some(6789));
    }

    #[test]
    fn parses_god_nodes() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("report-sample.md", &out);
        let r = read_graphify_report(&out).unwrap();
        assert_eq!(r.god_nodes.len(), 3);
        assert_eq!(r.god_nodes[0].rank, 1);
        assert_eq!(r.god_nodes[0].name, "GET()");
        assert_eq!(r.god_nodes[0].edge_count, 76);
        assert_eq!(r.god_nodes[2].name, "calculate_post_score()");
    }

    #[test]
    fn parses_surprising_connections() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("report-sample.md", &out);
        let r = read_graphify_report(&out).unwrap();
        assert_eq!(r.surprising_connections.len(), 3);
        let first = &r.surprising_connections[0];
        assert_eq!(first.source, "runCycle()");
        assert_eq!(first.target, "releaseStuckLocks()");
        assert_eq!(first.relation, "calls");
        assert_eq!(first.confidence, GraphifyConfidence::Inferred);
        assert_eq!(r.surprising_connections[1].confidence, GraphifyConfidence::Extracted);
        // 미지의 confidence -> Unknown
        assert_eq!(r.surprising_connections[2].confidence, GraphifyConfidence::Unknown);
    }

    #[test]
    fn parses_communities_with_cohesion_and_samples() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("report-sample.md", &out);
        let r = read_graphify_report(&out).unwrap();
        assert_eq!(r.communities.len(), 3);

        let c0 = &r.communities[0];
        assert_eq!(c0.id, 0);
        assert_eq!(c0.label, "Community 0");
        assert_eq!(c0.cohesion, Some(0.02));
        assert_eq!(c0.nodes_count, Some(71));
        assert_eq!(
            c0.sample_nodes,
            vec!["autoMapProxies()", "bulkImportAccounts()", "createAccount()"]
        );

        let c2 = &r.communities[2];
        assert_eq!(c2.label, "Custom label");
        assert_eq!(c2.cohesion, Some(1.0));
        assert_eq!(c2.nodes_count, Some(3));
        assert_eq!(c2.sample_nodes.len(), 3);
    }

    #[test]
    fn missing_section_yields_empty_vec() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        copy_fixture("report-minimal.md", &out);
        let r = read_graphify_report(&out).unwrap();
        assert!(r.god_nodes.is_empty());
        assert!(r.surprising_connections.is_empty());
        assert!(r.communities.is_empty());
    }

    #[test]
    fn strips_utf8_bom() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(
            b"# Graph Report - /a  (2026-04-26)\n\n## Summary\n- 5 nodes \xc2\xb7 6 edges \xc2\xb7 2 communities detected\n",
        );
        fs::write(out.join("GRAPH_REPORT.md"), &bytes).unwrap();
        let r = read_graphify_report(&out).unwrap();
        assert_eq!(r.project_root.as_deref(), Some("/a"));
        assert_eq!(r.summary.nodes_count, Some(5));
    }

    #[test]
    fn parses_real_bloghub_report_smoke() {
        // 실측 파일이 있을 때만 동작 (옵셔널)
        let real = PathBuf::from("/Users/uno/workspace/bloghub/graphify-out");
        if !real.join("GRAPH_REPORT.md").exists() {
            return;
        }
        let r = read_graphify_report(&real).expect("should parse real report");
        assert!(r.summary.nodes_count.unwrap_or(0) > 0);
        assert!(!r.god_nodes.is_empty());
        assert!(!r.communities.is_empty());
    }
}
