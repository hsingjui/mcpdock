mod commands;
mod db;
mod gateway;
mod mcp;
mod process_env;
mod state;

#[cfg(target_os = "macos")]
use tauri::RunEvent;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

use commands::capability::list_mcp_capabilities;
use commands::gateway::{get_gateway_status, restart_gateway};
use commands::group::{create_mcp_group, delete_mcp_group, list_mcp_groups, update_mcp_group};
use commands::import_export::{export_all_data, import_all_data};
use commands::install_mode::install_mode;
use commands::mcp::{
    call_mcp_tool, connect_mcp_server, create_mcp_server, delete_mcp_server, disconnect_mcp_server,
    get_mcp_runtime, list_mcp_servers, refresh_mcp_tools, toggle_mcp_server, update_mcp_server,
};
use commands::settings::{get_app_settings, update_app_settings};
use state::AppState;

/// Run the Tauri application.
///
/// # Panics
///
/// Panics if the application fails to build.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    process_env::repair_process_path();

    tauri::Builder::default()
        .setup(|app| {
            let db = db::init_db(app.handle())?;
            let settings = {
                let conn = db
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Failed to lock database"))?;
                db::app_settings::get_all(&conn)
            };
            // 开机自启模式：检测 --autostart 参数，根据设置决定是否隐藏窗口
            let is_autostart = std::env::args().any(|arg| arg == "--autostart");
            let start_hidden = is_autostart && settings.auto_start_hidden;
            let state = AppState::new(db, settings);
            app.manage(state);

            if is_autostart && start_hidden {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                    #[cfg(target_os = "macos")]
                    {
                        app.set_dock_visibility(false);
                        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                    }
                }
            }

            // macOS: 启动时显示 Dock 图标（带小圆点）
            #[cfg(target_os = "macos")]
            {
                let _: () = app.set_dock_visibility(true);
                app.set_activation_policy(tauri::ActivationPolicy::Regular);
            }

            // 设置系统托盘
            let show_item = MenuItemBuilder::with_id("show", "显示 MCPDock").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&quit_item)
                .build()?;

            // Windows: 彩色图标（托盘需要彩色才清晰可见）
            #[cfg(target_os = "windows")]
            let tray_icon =
                tauri::image::Image::from_bytes(include_bytes!("../icons/icon-tray-win.png"))
                    .expect("failed to load tray icon");
            #[cfg(not(target_os = "windows"))]
            let tray_icon =
                tauri::image::Image::from_bytes(include_bytes!("../icons/icon-tray.png"))
                    .expect("failed to load tray icon");

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&tray_menu)
                .tooltip("MCPDock")
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                                // macOS: 重新显示 Dock 图标（带小圆点）
                                #[cfg(target_os = "macos")]
                                {
                                    let _ = app.set_dock_visibility(true);
                                    let _ =
                                        app.set_activation_policy(tauri::ActivationPolicy::Regular);
                                }
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                            // macOS: 重新显示 Dock 图标（带小圆点）
                            #[cfg(target_os = "macos")]
                            {
                                let _ = app.set_dock_visibility(true);
                                let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
                            }
                        }
                    }
                })
                .build(app)?;

            // 窗口关闭时隐藏到托盘，不退出
            // macOS: 同时隐藏 Dock 图标（小圆点消失），只保留托盘图标
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                        // macOS: 隐藏 Dock 图标，仅保留系统托盘
                        #[cfg(target_os = "macos")]
                        {
                            let _ = window_clone.app_handle().set_dock_visibility(false);
                            let _ = window_clone
                                .app_handle()
                                .set_activation_policy(tauri::ActivationPolicy::Accessory);
                        }
                    }
                });
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    // Start the gateway immediately (before MCP connections)
                    match gateway::server::start_gateway(&app_handle, &state).await {
                        Ok(gateway_state) => {
                            let port = gateway_state.port;
                            *state.gateway.write().await = Some(gateway_state);
                            *state.gateway_error.write().await = None;
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
                            eprintln!("Failed to start gateway: {e}");
                            let err_msg = e.to_string();
                            *state.gateway_error.write().await = Some(err_msg.clone());
                            let _ = app_handle.emit(
                                "gateway:status-changed",
                                gateway::server::GatewayStatus {
                                    running: false,
                                    port: None,
                                    error: Some(err_msg.clone()),
                                },
                            );
                            let _ = app_handle.emit("gateway:error", err_msg);
                        }
                    }

                    // Connect enabled MCP servers concurrently in the background
                    if let Err(e) = mcp::manager::connect_enabled_servers(&app_handle, &state).await
                    {
                        eprintln!("Failed to connect enabled servers: {e}");
                    }
                }
            });
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--autostart"])
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 第二实例启动时，将已有窗口弹出并聚焦
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
                // macOS: 恢复 Dock 图标
                #[cfg(target_os = "macos")]
                {
                    let _ = app.set_dock_visibility(true);
                    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
                }
            }
        }))
        .invoke_handler(tauri::generate_handler![
            list_mcp_servers,
            create_mcp_server,
            update_mcp_server,
            delete_mcp_server,
            toggle_mcp_server,
            connect_mcp_server,
            disconnect_mcp_server,
            get_mcp_runtime,
            refresh_mcp_tools,
            call_mcp_tool,
            list_mcp_capabilities,
            list_mcp_groups,
            create_mcp_group,
            update_mcp_group,
            delete_mcp_group,
            get_app_settings,
            update_app_settings,
            get_gateway_status,
            restart_gateway,
            install_mode,
            export_all_data,
            import_all_data,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            #[cfg(target_os = "macos")]
            {
                if let RunEvent::Reopen { .. } = event {
                    // macOS: 点击 Dock 图标重新激活应用时，恢复窗口和 Dock 图标
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = app_handle.set_dock_visibility(true);
                        let _ = app_handle.set_activation_policy(tauri::ActivationPolicy::Regular);
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (app_handle, event);
            }
        });
}
