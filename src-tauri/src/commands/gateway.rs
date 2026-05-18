use tauri::{AppHandle, Emitter, State};

use crate::gateway::server;
use crate::state::AppState;

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn get_gateway_status(
    state: State<'_, AppState>,
) -> Result<server::GatewayStatus, String> {
    let gateway = state.gateway.read().await;
    if let Some(g) = gateway.as_ref() {
        Ok(server::GatewayStatus {
            running: true,
            port: Some(g.port),
            error: None,
        })
    } else {
        let error = state.gateway_error.read().await.clone();
        Ok(server::GatewayStatus {
            running: false,
            port: None,
            error,
        })
    }
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn restart_gateway(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<server::GatewayStatus, String> {
    match server::restart_gateway(&app_handle, &state).await {
        Ok(gateway_state) => {
            let port = gateway_state.port;
            *state.gateway.write().await = Some(gateway_state);
            *state.gateway_error.write().await = None;
            let status = server::GatewayStatus {
                running: true,
                port: Some(port),
                error: None,
            };
            let _ = app_handle.emit("gateway:status-changed", &status);
            Ok(status)
        }
        Err(e) => {
            *state.gateway_error.write().await = Some(e.to_string());
            let status = server::GatewayStatus {
                running: false,
                port: None,
                error: Some(e.to_string()),
            };
            let _ = app_handle.emit("gateway:status-changed", &status);
            Err(e.to_string())
        }
    }
}
