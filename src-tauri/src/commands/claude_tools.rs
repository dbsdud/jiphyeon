use tauri::State;

use crate::claude::{collect_claude_tools, ClaudeTools};
use crate::config::ConfigState;
use crate::error::AppError;

#[tauri::command]
pub fn get_claude_tools(
    config_state: State<'_, ConfigState>,
) -> Result<ClaudeTools, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let vault_path = config
        .vault_path
        .as_ref()
        .ok_or(AppError::VaultNotConfigured)?;
    Ok(collect_claude_tools(vault_path))
}
