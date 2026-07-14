//! Centralized main-window lifecycle management.
//!
//! When "low-resource background mode" is enabled (the default), closing the
//! main window destroys the WebView to release UI resources while the
//! background gateway, MCP runtimes and SQLite keep running. The window can be
//! recreated on demand from `tauri.conf.json` via the tray, single-instance or
//! macOS Dock activation paths.
//!
//! All restore paths share [`show_or_create_main_window`] so behavior stays
//! consistent.

use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Manager, WebviewWindow, WindowEvent};

use crate::state::AppState;

/// Label of the main window as declared in `tauri.conf.json`.
pub const MAIN_LABEL: &str = "main";

/// Show, unminimize and focus the main window, recreating it from
/// `tauri.conf.json` if it was destroyed by low-resource mode.
///
/// On macOS the Dock icon and Regular activation policy are restored.
pub fn show_or_create_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        restore_dock(app);
        return;
    }

    match create_main_window(app) {
        Ok(window) => {
            install_close_handler(&window);
            let _ = window.show();
            let _ = window.set_focus();
            restore_dock(app);
        }
        Err(e) => {
            eprintln!("Failed to recreate main window: {e}");
        }
    }
}

/// Recreate the main window from the `tauri.conf.json` window config whose
/// label matches [`MAIN_LABEL`].
fn create_main_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let config = app.config();
    let window_config = config
        .app
        .windows
        .iter()
        .find(|w| w.label == MAIN_LABEL)
        .ok_or(tauri::Error::WindowNotFound)?;

    let window = WebviewWindowBuilder::from_config(app, window_config)?.build()?;
    Ok(window)
}

/// Install the close-requested handler that either hides or destroys the
/// window depending on the persisted setting.
pub fn install_close_handler(window: &WebviewWindow) {
    let window_clone = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            // Hide immediately so the user does not see a destroy delay, and
            // so a concurrent restore request can win the race.
            let _ = window_clone.hide();
            #[cfg(target_os = "macos")]
            {
                let _ = window_clone.app_handle().set_dock_visibility(false);
                let _ = window_clone
                    .app_handle()
                    .set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            let app = window_clone.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                let enabled = read_low_resource_enabled(&app).await;
                if !enabled {
                    return;
                }
                // If the user restored the window before we got here, skip the
                // destroy so we never tear down a visible UI.
                if let Some(w) = app.get_webview_window(MAIN_LABEL) {
                    if w.is_visible().unwrap_or(false) {
                        return;
                    }
                    if let Err(e) = w.destroy() {
                        eprintln!("Failed to destroy main window: {e}");
                    }
                }
            });
        }
    });
}

/// Read the persisted low-resource setting, falling back to `true` (enabled)
/// on any error so the default behavior is resource-saving.
async fn read_low_resource_enabled(app: &AppHandle) -> bool {
    match app.try_state::<AppState>() {
        Some(state) => state.settings.read().await.low_resource_mode_enabled,
        None => true,
    }
}

/// Destroy the main window immediately, used for the auto-start-hidden path
/// when low-resource mode is enabled.
pub fn destroy_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        if let Err(e) = window.destroy() {
            eprintln!("Failed to destroy main window during startup: {e}");
        }
    }
}

#[cfg(target_os = "macos")]
fn restore_dock(app: &AppHandle) {
    let _ = app.set_dock_visibility(true);
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
}

#[cfg(not(target_os = "macos"))]
const fn restore_dock(_app: &AppHandle) {}
