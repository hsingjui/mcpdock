use tauri::{AppHandle, Emitter, State};

use crate::db::mcp_group::{self, McpGroup, McpGroupInput};
use crate::gateway;
use crate::state::AppState;

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn list_mcp_groups(state: State<'_, AppState>) -> Result<Vec<McpGroup>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "Failed to lock database".to_string())?;
    mcp_group::list(&db).map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn create_mcp_group(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: McpGroupInput,
) -> Result<McpGroup, String> {
    let group = {
        let db = state
            .db
            .lock()
            .map_err(|_| "Failed to lock database".to_string())?;
        mcp_group::create(&db, &input).map_err(|e| e.to_string())?
    };

    // Restart gateway to pick up the new group
    restart_gateway_notify(&app_handle, &state).await;

    Ok(group)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn update_mcp_group(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: String,
    input: McpGroupInput,
) -> Result<McpGroup, String> {
    let group = {
        let db = state
            .db
            .lock()
            .map_err(|_| "Failed to lock database".to_string())?;
        mcp_group::update(&db, &id, &input).map_err(|e| e.to_string())?
    };

    // Restart gateway to pick up the updated group
    restart_gateway_notify(&app_handle, &state).await;

    Ok(group)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn delete_mcp_group(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    {
        let db = state
            .db
            .lock()
            .map_err(|_| "Failed to lock database".to_string())?;
        mcp_group::delete(&db, &id).map_err(|e| e.to_string())?;
    }

    // Restart gateway to pick up the deletion
    restart_gateway_notify(&app_handle, &state).await;

    Ok(())
}

/// Restart gateway and notify frontend of status changes.
async fn restart_gateway_notify(app_handle: &AppHandle, state: &AppState) {
    match gateway::server::restart_gateway(app_handle, state).await {
        Ok(gateway_state) => {
            let port = gateway_state.port;
            *state.gateway.write().await = Some(gateway_state);
            let _ = app_handle.emit(
                "gateway:status-changed",
                gateway::server::GatewayStatus {
                    running: true,
                    port: Some(port),
                    error: None,
                },
            );
        }
        Err(e) => {
            eprintln!("Failed to restart gateway after group change: {e}");
            *state.gateway.write().await = None;
            let _ = app_handle.emit(
                "gateway:status-changed",
                gateway::server::GatewayStatus {
                    running: false,
                    port: None,
                    error: Some(e.to_string()),
                },
            );
            let _ = app_handle.emit("gateway:error", e.to_string());
        }
    }
}
