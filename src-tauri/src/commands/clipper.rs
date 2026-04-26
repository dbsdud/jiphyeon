use tauri::State;

use crate::clipper;
use crate::config::ConfigState;
use crate::error::AppError;
use crate::models::{ClipRequest, ClipResult};

#[tauri::command]
pub fn clip_url(
    config_state: State<'_, ConfigState>,
    request: ClipRequest,
) -> Result<ClipResult, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let project = config.active_project().ok_or(AppError::VaultNotConfigured)?;
    clipper::clip_url(&request, &project.docs_path)
}
