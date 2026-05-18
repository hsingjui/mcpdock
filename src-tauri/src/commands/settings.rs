use tauri::{AppHandle, Emitter, State};

use crate::db::app_settings;
use crate::gateway;
use crate::state::AppState;

/// Gateway-affecting fields — only restart gateway when one of these changes.
fn gateway_relevant_changed(
    old: &app_settings::AppSettings,
    new: &app_settings::AppSettings,
) -> bool {
    old.port != new.port
        || old.proxy_url != new.proxy_url
        || old.auth_enabled != new.auth_enabled
        || old.auth_token != new.auth_token
        || old.request_timeout_enabled != new.request_timeout_enabled
        || old.request_timeout_ms != new.request_timeout_ms
        || old.keep_alive_enabled != new.keep_alive_enabled
        || old.keep_alive_interval_ms != new.keep_alive_interval_ms
        || old.gateway_separator != new.gateway_separator
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn get_app_settings(
    state: State<'_, AppState>,
) -> Result<app_settings::AppSettings, String> {
    let settings = state.settings.read().await;
    Ok(settings.clone())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn update_app_settings(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    settings: app_settings::AppSettings,
) -> Result<app_settings::AppSettings, String> {
    // Capture old settings before overwrite
    let old_settings = {
        let guard = state.settings.read().await;
        guard.clone()
    };

    // Write to DB - drop the lock before awaiting
    {
        let db = state
            .db
            .lock()
            .map_err(|_| "Failed to lock database".to_string())?;
        app_settings::update_all(&db, &settings).map_err(|e| e.to_string())?;
    }

    // Update in-memory cache
    {
        let mut guard = state.settings.write().await;
        *guard = settings;
    }

    // Re-read from DB to confirm
    let updated = {
        let db = state
            .db
            .lock()
            .map_err(|_| "Failed to lock database".to_string())?;
        app_settings::get_all(&db)
    };

    // Only restart gateway when gateway-relevant settings actually changed
    if gateway_relevant_changed(&old_settings, &updated) {
        match gateway::server::restart_gateway(&app_handle, &state).await {
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
                eprintln!("Failed to restart gateway: {e}");
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

    Ok(updated)
}
