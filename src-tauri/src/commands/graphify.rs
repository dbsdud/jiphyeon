//! 활성 프로젝트의 graphify-out 을 노출하는 IPC.

use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::State;

use crate::config::ConfigState;
use crate::error::AppError;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::config::AppConfig;
use crate::graphify::cross::{
    merge_graphs, CrossProjectGraph, CrossProjectMember,
};
use crate::graphify::reader::{read_graphify_graph, GraphifyError, GraphifyGraph};
use crate::graphify::report::{read_graphify_report, GraphReport};
use crate::project::ProjectEntry;

const DEFAULT_EXCLUDE_TREE_DIRS_PENDING: &[&str] =
    &[".git", ".claude", "node_modules", "target", "graphify-out"];

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PendingStatus {
    Fresh,
    Stale,
    NotRun,
    NoProject,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PendingGraphify {
    pub project_id: Option<String>,
    pub status: PendingStatus,
    pub graph_run_at: Option<i64>,
    pub docs_changed_at: Option<i64>,
    pub changed_files_count: usize,
}

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
pub fn get_pending_graphify(
    state: State<'_, ConfigState>,
) -> Result<PendingGraphify, AppError> {
    let (project, exclude_dirs) = {
        let config = state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        (config.active_project().cloned(), config.exclude_dirs.clone())
    };
    Ok(compute_pending_graphify(project.as_ref(), &exclude_dirs))
}

pub fn compute_pending_graphify(
    project: Option<&ProjectEntry>,
    exclude_dirs: &[String],
) -> PendingGraphify {
    let Some(project) = project else {
        return PendingGraphify {
            project_id: None,
            status: PendingStatus::NoProject,
            graph_run_at: None,
            docs_changed_at: None,
            changed_files_count: 0,
        };
    };

    let project_id = Some(project.id.clone());
    let graph_json = project.graphify_out_path.join("graph.json");
    let graph_run_at = match fs::metadata(&graph_json) {
        Ok(m) => m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64),
        Err(_) => {
            return PendingGraphify {
                project_id,
                status: PendingStatus::NotRun,
                graph_run_at: None,
                docs_changed_at: None,
                changed_files_count: 0,
            };
        }
    };

    let mut excluded: Vec<String> = exclude_dirs.to_vec();
    for d in DEFAULT_EXCLUDE_TREE_DIRS_PENDING {
        excluded.push(d.to_string());
    }

    let mut docs_changed_at: Option<i64> = None;
    let mut changed_files_count = 0usize;
    if project.docs_path.is_dir() {
        scan_docs_mtimes(
            &project.docs_path,
            &excluded,
            graph_run_at,
            &mut docs_changed_at,
            &mut changed_files_count,
        );
    }

    let status = if changed_files_count > 0 {
        PendingStatus::Stale
    } else {
        PendingStatus::Fresh
    };

    PendingGraphify {
        project_id,
        status,
        graph_run_at,
        docs_changed_at,
        changed_files_count,
    }
}

fn scan_docs_mtimes(
    dir: &Path,
    exclude_dirs: &[String],
    graph_run_at: Option<i64>,
    docs_changed_at: &mut Option<i64>,
    changed_files_count: &mut usize,
) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if path.is_dir() {
            if file_name.starts_with('.') {
                continue;
            }
            if exclude_dirs.iter().any(|d| d == file_name) {
                continue;
            }
            scan_docs_mtimes(
                &path,
                exclude_dirs,
                graph_run_at,
                docs_changed_at,
                changed_files_count,
            );
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if ext != "md" {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        match docs_changed_at {
            Some(prev) if *prev >= mtime => {}
            _ => *docs_changed_at = Some(mtime),
        }
        if let Some(grt) = graph_run_at {
            if mtime > grt {
                *changed_files_count += 1;
            }
        }
    }
}

// 미사용이지만 향후 비활성 프로젝트 다중 조회 등에 활용 가능
#[allow(dead_code)]
fn config_active_project_clone(config: &AppConfig) -> Option<ProjectEntry> {
    config.active_project().cloned()
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

    // --- compute_pending_graphify ---

    fn touch_md_with_offset(path: &Path, offset_secs: i64) {
        use std::time::{Duration, SystemTime};
        fs::write(path, "x").unwrap();
        let target = if offset_secs >= 0 {
            SystemTime::now() + Duration::from_secs(offset_secs as u64)
        } else {
            SystemTime::now() - Duration::from_secs((-offset_secs) as u64)
        };
        let _ = filetime::set_file_mtime(
            path,
            filetime::FileTime::from_system_time(target),
        );
    }

    #[test]
    fn pending_no_project() {
        let p = compute_pending_graphify(None, &[]);
        assert_eq!(p.status, PendingStatus::NoProject);
        assert!(p.project_id.is_none());
    }

    #[test]
    fn pending_not_run_when_graph_json_missing() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        let entry = entry_with_dir(dir.path().to_path_buf());
        let p = compute_pending_graphify(Some(&entry), &[]);
        assert_eq!(p.status, PendingStatus::NotRun);
        assert_eq!(p.changed_files_count, 0);
    }

    #[test]
    fn pending_fresh_when_no_md_changes() {
        let dir = TempDir::new().unwrap();
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        // 먼저 docs 파일, 그 다음 graph.json 을 나중에 만들면 graph_run_at >= mtime
        touch_md_with_offset(&docs.join("a.md"), -120); // 2분 전
        fs::write(out.join("graph.json"), "{}").unwrap();

        let entry = entry_with_dir(dir.path().to_path_buf());
        let p = compute_pending_graphify(Some(&entry), &[]);
        assert_eq!(p.status, PendingStatus::Fresh);
        assert_eq!(p.changed_files_count, 0);
    }

    #[test]
    fn pending_stale_when_md_newer_than_graph() {
        let dir = TempDir::new().unwrap();
        let docs = dir.path().join("docs");
        fs::create_dir_all(&docs).unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        // graph.json 을 과거로, docs 파일을 미래 mtime 으로
        fs::write(out.join("graph.json"), "{}").unwrap();
        let _ = filetime::set_file_mtime(
            out.join("graph.json"),
            filetime::FileTime::from_unix_time(
                (std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64)
                    - 600,
                0,
            ),
        );
        touch_md_with_offset(&docs.join("a.md"), 60);
        touch_md_with_offset(&docs.join("b.md"), 60);

        let entry = entry_with_dir(dir.path().to_path_buf());
        let p = compute_pending_graphify(Some(&entry), &[]);
        assert_eq!(p.status, PendingStatus::Stale);
        assert_eq!(p.changed_files_count, 2);
    }

    #[test]
    fn pending_skips_excluded_dirs() {
        let dir = TempDir::new().unwrap();
        let docs = dir.path().join("docs");
        fs::create_dir_all(docs.join("node_modules")).unwrap();
        fs::create_dir_all(&docs).unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("graph.json"), "{}").unwrap();
        let _ = filetime::set_file_mtime(
            out.join("graph.json"),
            filetime::FileTime::from_unix_time(
                (std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64)
                    - 600,
                0,
            ),
        );
        touch_md_with_offset(&docs.join("node_modules").join("hide.md"), 60);

        let entry = entry_with_dir(dir.path().to_path_buf());
        let exclude = vec!["node_modules".to_string()];
        let p = compute_pending_graphify(Some(&entry), &exclude);
        assert_eq!(p.status, PendingStatus::Fresh);
        assert_eq!(p.changed_files_count, 0);
    }

    #[test]
    fn pending_recurses_into_subdirs() {
        let dir = TempDir::new().unwrap();
        let docs = dir.path().join("docs");
        let sub = docs.join("decisions");
        fs::create_dir_all(&sub).unwrap();
        let out = dir.path().join("graphify-out");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("graph.json"), "{}").unwrap();
        let _ = filetime::set_file_mtime(
            out.join("graph.json"),
            filetime::FileTime::from_unix_time(
                (std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64)
                    - 600,
                0,
            ),
        );
        touch_md_with_offset(&sub.join("nested.md"), 60);

        let entry = entry_with_dir(dir.path().to_path_buf());
        let p = compute_pending_graphify(Some(&entry), &[]);
        assert_eq!(p.status, PendingStatus::Stale);
        assert_eq!(p.changed_files_count, 1);
    }
}
