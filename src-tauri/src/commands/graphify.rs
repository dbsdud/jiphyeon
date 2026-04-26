//! 활성 프로젝트의 graphify-out 을 노출하는 IPC.

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::State;

use crate::config::ConfigState;
use crate::error::AppError;
use crate::graphify::cross::{
    merge_graphs, CrossProjectGraph, CrossProjectMember,
};
use crate::graphify::reader::{read_graphify_graph, GraphifyError, GraphifyGraph};
use crate::graphify::report::{read_graphify_report, GraphReport};
use crate::project::ProjectEntry;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct GraphifyStatus {
    pub project_id: Option<String>,
    pub graphify_out_path: Option<String>,
    pub graph_json_exists: bool,
    pub report_md_exists: bool,
    pub last_run_at: Option<String>,
    pub nodes_count: Option<usize>,
    pub edges_count: Option<usize>,
}

#[tauri::command]
pub fn get_graphify_graph(
    state: State<'_, ConfigState>,
) -> Result<GraphifyGraph, GraphifyError> {
    let path = active_graphify_out(&state)?;
    read_graphify_graph(&path)
}

#[tauri::command]
pub fn get_graphify_report(
    state: State<'_, ConfigState>,
) -> Result<GraphReport, GraphifyError> {
    let path = active_graphify_out(&state)?;
    read_graphify_report(&path)
}

#[tauri::command]
pub fn get_cross_project_graph(
    state: State<'_, ConfigState>,
    project_ids: Vec<String>,
    merge_labels: bool,
) -> Result<CrossProjectGraph, GraphifyError> {
    let projects: Vec<ProjectEntry> = {
        let config = state
            .read()
            .map_err(|_| GraphifyError::NotRun)?;
        if project_ids.is_empty() {
            config.projects.clone()
        } else {
            config
                .projects
                .iter()
                .filter(|p| project_ids.contains(&p.id))
                .cloned()
                .collect()
        }
    };

    let mut members: Vec<CrossProjectMember> = Vec::new();
    let mut graphs: Vec<(String, GraphifyGraph)> = Vec::new();
    for project in &projects {
        match read_graphify_graph(&project.graphify_out_path) {
            Ok(g) => {
                members.push(CrossProjectMember {
                    project_id: project.id.clone(),
                    project_name: project.name.clone(),
                });
                graphs.push((project.id.clone(), g));
            }
            Err(GraphifyError::NotRun) => {
                // graphify 미실행 프로젝트는 스킵
                continue;
            }
            Err(e) => {
                eprintln!(
                    "cross-project graph: {} 파싱 실패 ({}): {e}",
                    project.name,
                    project.graphify_out_path.display()
                );
                continue;
            }
        }
    }

    if members.is_empty() {
        return Err(GraphifyError::NotRun);
    }

    Ok(merge_graphs(&members, graphs, merge_labels))
}

#[tauri::command]
pub fn get_graphify_status(
    state: State<'_, ConfigState>,
) -> Result<GraphifyStatus, AppError> {
    let project = {
        let config = state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        config.active_project().cloned()
    };
    Ok(compute_status(project.as_ref()))
}

/// 활성 프로젝트의 graphify_out_path. 없으면 NotRun.
fn active_graphify_out(state: &State<'_, ConfigState>) -> Result<std::path::PathBuf, GraphifyError> {
    let config = state
        .read()
        .map_err(|_| GraphifyError::NotRun)?;
    config
        .active_project()
        .map(|p| p.graphify_out_path.clone())
        .ok_or(GraphifyError::NotRun)
}

pub fn compute_status(project: Option<&ProjectEntry>) -> GraphifyStatus {
    let Some(project) = project else {
        return GraphifyStatus::default();
    };

    let dir = &project.graphify_out_path;
    let graph_json = dir.join("graph.json");
    let report_md = dir.join("GRAPH_REPORT.md");

    let graph_json_exists = graph_json.is_file();
    let report_md_exists = report_md.is_file();

    let last_run_at = if graph_json_exists {
        std::fs::metadata(&graph_json)
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t: SystemTime| {
                let dt: DateTime<Utc> = t.into();
                dt.to_rfc3339()
            })
    } else {
        None
    };

    let (nodes_count, edges_count) = if graph_json_exists {
        match read_graphify_graph(dir) {
            Ok(g) => (Some(g.nodes.len()), Some(g.edges.len())),
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    GraphifyStatus {
        project_id: Some(project.id.clone()),
        graphify_out_path: Some(dir.to_string_lossy().to_string()),
        graph_json_exists,
        report_md_exists,
        last_run_at,
        nodes_count,
        edges_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::new_project_entry;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn entry_with_dir(root: PathBuf) -> ProjectEntry {
        new_project_entry(root.clone(), root, None)
    }

    #[test]
    fn status_no_active_project_is_empty() {
        let s = compute_status(None);
        assert!(s.project_id.is_none());
        assert!(!s.graph_json_exists);
        assert!(!s.report_md_exists);
        assert!(s.last_run_at.is_none());
        assert!(s.nodes_count.is_none());
    }

    #[test]
    fn status_active_but_no_graphify_out() {
        let dir = TempDir::new().unwrap();
        let entry = entry_with_dir(dir.path().to_path_buf());
        let s = compute_status(Some(&entry));
        assert_eq!(s.project_id, Some(entry.id.clone()));
        assert!(s.graphify_out_path.is_some());
        assert!(!s.graph_json_exists);
        assert!(!s.report_md_exists);
        assert!(s.last_run_at.is_none());
        assert!(s.nodes_count.is_none());
    }

    #[test]
    fn status_with_graph_json_only() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        fs::write(
            out.join("graph.json"),
            r#"{"directed":false,"multigraph":false,"graph":{},"nodes":[{"id":"a"}],"links":[]}"#,
        )
        .unwrap();

        let entry = entry_with_dir(dir.path().to_path_buf());
        let s = compute_status(Some(&entry));
        assert!(s.graph_json_exists);
        assert!(!s.report_md_exists);
        assert!(s.last_run_at.is_some());
        assert_eq!(s.nodes_count, Some(1));
        assert_eq!(s.edges_count, Some(0));
    }

    #[test]
    fn status_with_both_files() {
        let dir = TempDir::new().unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        fs::write(
            out.join("graph.json"),
            r#"{"directed":false,"multigraph":false,"graph":{},"nodes":[{"id":"a"},{"id":"b"}],"links":[{"source":"a","target":"b","relation":"x","confidence":"EXTRACTED","confidence_score":1.0,"weight":1.0}]}"#,
        )
        .unwrap();
        fs::write(out.join("GRAPH_REPORT.md"), "# Graph Report - /x  (2026-04-26)").unwrap();

        let entry = entry_with_dir(dir.path().to_path_buf());
        let s = compute_status(Some(&entry));
        assert!(s.graph_json_exists);
        assert!(s.report_md_exists);
        assert_eq!(s.nodes_count, Some(2));
        assert_eq!(s.edges_count, Some(1));
    }
}
