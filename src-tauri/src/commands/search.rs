//! 통합 검색 IPC.

use std::sync::{Arc, RwLock};

use tauri::State;

use crate::config::ConfigState;
use crate::error::AppError;
use crate::project::ProjectEntry;
use crate::search::{
    reindex_all as reindex_all_impl, reindex_project, search as search_impl, SearchError,
    SearchHit, SearchIndex, SearchKind,
};

pub type SearchState = Arc<RwLock<SearchIndex>>;

#[tauri::command]
pub fn search(
    state: State<'_, SearchState>,
    query: String,
    project_filter: Option<Vec<String>>,
    kind_filter: Option<Vec<SearchKind>>,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>, SearchError> {
    let idx = state
        .read()
        .map_err(|e| SearchError::Tantivy(e.to_string()))?;
    let limit = limit.unwrap_or(50);
    let pf = project_filter.as_deref();
    let kf = kind_filter.as_deref();
    search_impl(&idx, &query, pf, kf, limit)
}

#[tauri::command]
pub fn reindex_active_project(
    config_state: State<'_, ConfigState>,
    search_state: State<'_, SearchState>,
) -> Result<usize, SearchError> {
    let project: Option<ProjectEntry> = {
        let cfg = config_state
            .read()
            .map_err(|e| SearchError::Tantivy(e.to_string()))?;
        cfg.active_project().cloned()
    };
    let Some(project) = project else { return Ok(0) };
    let idx = search_state
        .read()
        .map_err(|e| SearchError::Tantivy(e.to_string()))?;
    reindex_project(&idx, &project)
}

#[tauri::command]
pub fn reindex_all_projects(
    config_state: State<'_, ConfigState>,
    search_state: State<'_, SearchState>,
) -> Result<usize, AppError> {
    let projects: Vec<ProjectEntry> = {
        let cfg = config_state
            .read()
            .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
        cfg.projects.clone()
    };
    let idx = search_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    reindex_all_impl(&idx, &projects).map_err(|e| AppError::VaultNotFound(e.to_string()))
}
