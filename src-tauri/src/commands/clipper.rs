use tauri::State;

use crate::clipper;
use crate::config::ConfigState;
use crate::error::AppError;
use crate::models::{ClipRequest, ClipResult};

#[tauri::command]
pub fn clip_url(
    config_state: State<'_, ConfigState>,
    request: ClipRequest,
    project_id: Option<String>,
) -> Result<ClipResult, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let project = match project_id.as_deref() {
        Some(id) => config
            .projects
            .iter()
            .find(|p| p.id == id)
            .ok_or(AppError::VaultNotConfigured)?,
        None => config.active_project().ok_or(AppError::VaultNotConfigured)?,
    };
    clipper::clip_url(&request, &project.docs_path)
}
