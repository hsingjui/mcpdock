use tauri::State;

use crate::db::mcp_capability::{self, McpCapability};
use crate::state::AppState;

#[allow(clippy::needless_pass_by_value, clippy::option_if_let_else)]
#[tauri::command]
pub fn list_mcp_capabilities(
    state: State<'_, AppState>,
    server_id: Option<i64>,
) -> Result<Vec<McpCapability>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "Failed to lock database".to_string())?;
    if let Some(sid) = server_id {
        mcp_capability::list_by_server(&db, sid)
    } else {
        mcp_capability::list_all(&db)
    }
    .map_err(|e| e.to_string())
}
