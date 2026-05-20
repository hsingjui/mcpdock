use std::collections::HashMap;

use anyhow::Error;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::db::mcp_server::{self, McpServerInput, McpServerRow};
use crate::db::{mcp_capability, mcp_group};
use crate::gateway;
use crate::mcp::manager;
use crate::mcp::runtime::McpServerRuntime;
use crate::state::AppState;

fn format_error_chain(error: Error) -> String {
    format!("{error:#}")
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<serde_json::Value>,
    pub is_error: Option<bool>,
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn list_mcp_servers(state: State<'_, AppState>) -> Result<Vec<McpServerRow>, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "Failed to lock database".to_string())?;
    mcp_server::list(&db).map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn create_mcp_server(
    state: State<'_, AppState>,
    input: McpServerInput,
) -> Result<McpServerRow, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "Failed to lock database".to_string())?;
    mcp_server::create(&db, &input).map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn update_mcp_server(
    state: State<'_, AppState>,
    id: i64,
    input: McpServerInput,
) -> Result<McpServerRow, String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "Failed to lock database".to_string())?;
    mcp_server::update(&db, id, &input).map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn delete_mcp_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let _ = manager::disconnect(&app_handle, &state, id).await;
    {
        let db = state
            .db
            .lock()
            .map_err(|_| "Failed to lock database".to_string())?;
        let _ = mcp_capability::delete_by_server(&db, id).map_err(|e: anyhow::Error| e.to_string());
        let _ = mcp_group::remove_server_from_all_groups(&db, id)
            .map_err(|e: anyhow::Error| e.to_string());
        mcp_server::delete(&db, id).map_err(|e| e.to_string())?;
    }

    // Restart gateway to pick up any group config changes caused by server removal
    restart_gateway_notify(&app_handle, &state).await;

    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn toggle_mcp_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<McpServerRow, String> {
    let row = {
        let db = state
            .db
            .lock()
            .map_err(|_| "Failed to lock database".to_string())?;
        mcp_server::toggle(&db, id).map_err(|e| e.to_string())?
    };

    if row.enabled {
        let _ = manager::connect(&app_handle, &state, id).await;
    } else {
        let _ = manager::disconnect(&app_handle, &state, id).await;
    }

    Ok(row)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn connect_mcp_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<McpServerRuntime, String> {
    manager::connect(&app_handle, &state, id)
        .await
        .map_err(format_error_chain)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn disconnect_mcp_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<McpServerRuntime, String> {
    manager::disconnect(&app_handle, &state, id)
        .await
        .map_err(format_error_chain)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn get_mcp_runtime(
    state: State<'_, AppState>,
    id: Option<i64>,
) -> Result<HashMap<i64, McpServerRuntime>, String> {
    match id {
        Some(server_id) => {
            let runtime = manager::get_runtime(&state, server_id).map_err(format_error_chain)?;
            Ok(HashMap::from([(server_id, runtime)]))
        }
        None => manager::list_runtimes(&state).map_err(format_error_chain),
    }
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn refresh_mcp_tools(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<McpServerRuntime, String> {
    manager::refresh(&app_handle, &state, id)
        .await
        .map_err(format_error_chain)
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn call_mcp_tool(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    tool_name: String,
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<CallToolResult, String> {
    let result = manager::call_tool(&app_handle, &state, id, &tool_name, arguments)
        .await
        .map_err(format_error_chain)?;
    Ok(result)
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
            eprintln!("Failed to restart gateway after server deletion: {e}");
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
