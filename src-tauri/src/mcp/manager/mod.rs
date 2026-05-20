mod discovery;
mod runtime_state;
mod transport;

use std::time::Duration;

use anyhow::{bail, Context};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::commands::mcp::CallToolResult;
use crate::db::mcp_capability;
use crate::db::mcp_server::McpServerRow;
use crate::mcp::runtime::{McpClientHolder, McpServerRuntime};
use crate::state::AppState;

pub use runtime_state::{get_runtime, list_runtimes};

pub async fn connect(
    app_handle: &AppHandle,
    state: &AppState,
    server_id: i64,
) -> anyhow::Result<McpServerRuntime> {
    let server = load_server(state, server_id)?;

    if !server.enabled {
        bail!("MCP server {server_id} is disabled");
    }

    let _ = disconnect(app_handle, state, server_id).await;

    runtime_state::update_runtime(state, server_id, |runtime| {
        runtime.connecting = true;
        runtime.connected = false;
        runtime.error = None;
    })?;
    runtime_state::emit_runtime(app_handle, state, server_id)?;

    let timeout_duration = {
        let settings = state.settings.read().await;
        if settings.request_timeout_enabled {
            Some(Duration::from_millis(settings.request_timeout_ms))
        } else {
            None
        }
    };

    let client_result = match server.transport_type.as_str() {
        "stdio" => {
            let fut = transport::connect_stdio(&server);
            match timeout_duration {
                Some(dur) => tokio::time::timeout(dur, fut)
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!("Connection timed out after {}ms", dur.as_millis())
                    })?
                    .context("Failed to connect stdio MCP server"),
                None => fut.await.context("Failed to connect stdio MCP server"),
            }
        }
        "streamable_http" => {
            // connect_streamable_http applies timeout internally for the handshake
            transport::connect_streamable_http(state, &server).await
        }
        other => bail!("Unsupported transport type: {other}"),
    };

    match client_result {
        Ok(client) => {
            let discovered = discovery::discover_capabilities(&client, state).await?;
            {
                let db = state
                    .db
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Failed to lock database"))?;
                mcp_capability::replace_server_capabilities(
                    &db,
                    server_id,
                    &discovered.tools,
                    &discovered.prompts,
                    &discovered.resources,
                    &discovered.resource_templates,
                )?;
            }

            let connection_id = Uuid::new_v4();
            let monitor_client = client.peer().clone();
            let keep_alive_enabled = {
                let settings = state.settings.read().await;
                settings.keep_alive_enabled
            };

            // Start keep-alive ping task if enabled
            let keep_alive_handle = if keep_alive_enabled {
                let peer = client.peer().clone();
                let interval = {
                    let settings = state.settings.read().await;
                    Duration::from_millis(settings.keep_alive_interval_ms)
                };
                let server_name = server.name.clone();
                Some(tokio::spawn(async move {
                    let mut ticker = tokio::time::interval(interval);
                    // First tick fires immediately, skip it
                    ticker.tick().await;
                    loop {
                        ticker.tick().await;
                        if let Err(e) = peer.list_all_tools().await {
                            eprintln!("Keep-alive ping failed for server '{server_name}': {e}");
                        }
                    }
                }))
            } else {
                None
            };

            let monitor_handle =
                spawn_runtime_monitor(app_handle.clone(), server_id, connection_id, monitor_client);

            let mut holder = McpClientHolder::new(client, connection_id);
            holder.keep_alive_handle = keep_alive_handle;
            holder.monitor_handle = Some(monitor_handle);
            {
                let mut clients = state.clients.lock().await;
                clients.insert(server_id, holder);
            }
            let runtime = runtime_state::set_runtime(
                state,
                McpServerRuntime {
                    server_id,
                    connected: true,
                    connecting: false,
                    error: None,
                    discovered_at: Some(chrono::Utc::now().timestamp_millis()),
                },
            )?;
            runtime_state::emit_runtime(app_handle, state, server_id)?;
            Ok(runtime)
        }
        Err(error) => {
            let _runtime = runtime_state::update_runtime(state, server_id, |runtime| {
                runtime.connecting = false;
                runtime.connected = false;
                runtime.error = Some(format!("{error:#}"));
            })?;
            runtime_state::emit_runtime(app_handle, state, server_id)?;
            Err(error)
        }
    }
}

pub async fn disconnect(
    app_handle: &AppHandle,
    state: &AppState,
    server_id: i64,
) -> anyhow::Result<McpServerRuntime> {
    let mut holder = {
        let mut clients = state.clients.lock().await;
        clients.remove(&server_id)
    };

    if let Some(holder) = holder.as_mut() {
        holder.close().await?;
    }

    let runtime = runtime_state::update_runtime(state, server_id, |runtime| {
        runtime.connected = false;
        runtime.connecting = false;
        runtime.error = None;
        runtime.discovered_at = None;
    })?;

    runtime_state::emit_runtime(app_handle, state, server_id)?;
    Ok(runtime)
}

pub async fn refresh(
    app_handle: &AppHandle,
    state: &AppState,
    server_id: i64,
) -> anyhow::Result<McpServerRuntime> {
    // Clone the peer and release the lock before discovery to avoid blocking
    // other operations on the clients map. The client stays in the map.
    let peer = {
        let clients = state.clients.lock().await;
        let holder = clients
            .get(&server_id)
            .ok_or_else(|| anyhow::anyhow!("MCP server {server_id} is not connected"))?;
        let client_ref = holder
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MCP server {server_id} client is unavailable"))?;
        client_ref.peer().clone()
    };

    let discovered = discovery::discover_capabilities_from_peer(&peer, state).await?;

    {
        let db = state
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to lock database"))?;
        mcp_capability::replace_server_capabilities(
            &db,
            server_id,
            &discovered.tools,
            &discovered.prompts,
            &discovered.resources,
            &discovered.resource_templates,
        )?;
    }

    let runtime = runtime_state::set_runtime(
        state,
        McpServerRuntime {
            server_id,
            connected: true,
            connecting: false,
            error: None,
            discovered_at: Some(chrono::Utc::now().timestamp_millis()),
        },
    )?;

    runtime_state::emit_runtime(app_handle, state, server_id)?;
    Ok(runtime)
}

pub async fn connect_enabled_servers(
    app_handle: &AppHandle,
    state: &AppState,
) -> anyhow::Result<()> {
    let servers = {
        let db = state
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to lock database"))?;
        crate::db::mcp_server::list_enabled(&db)?
    };

    // Connect all enabled servers concurrently
    let handles: Vec<_> = servers
        .into_iter()
        .map(|server| {
            let app_handle = app_handle.clone();
            tokio::spawn(async move {
                let state = app_handle.state::<AppState>();
                let _ = connect(&app_handle, &state, server.id).await;
            })
        })
        .collect();

    // Wait for all connections to complete
    for handle in handles {
        let _ = handle.await;
    }

    Ok(())
}

fn load_server(state: &AppState, server_id: i64) -> anyhow::Result<McpServerRow> {
    let db = state
        .db
        .lock()
        .map_err(|_| anyhow::anyhow!("Failed to lock database"))?;
    crate::db::mcp_server::get(&db, server_id)
}

pub async fn call_tool(
    app_handle: &AppHandle,
    state: &AppState,
    server_id: i64,
    tool_name: &str,
    arguments: Option<serde_json::Map<String, serde_json::Value>>,
) -> anyhow::Result<CallToolResult> {
    let timeout_duration = {
        let settings = state.settings.read().await;
        if settings.request_timeout_enabled {
            Some(Duration::from_millis(settings.request_timeout_ms))
        } else {
            None
        }
    };

    // Clone the peer and release the lock before awaiting to avoid blocking
    // other operations on the clients map.
    let peer = {
        let clients = state.clients.lock().await;
        let holder = clients
            .get(&server_id)
            .ok_or_else(|| anyhow::anyhow!("MCP server {server_id} is not connected"))?;
        let client_ref = holder
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MCP server {server_id} client is unavailable"))?;
        client_ref.peer().clone()
    };

    let params = rmcp::model::CallToolRequestParams::new(tool_name.to_string())
        .with_arguments(arguments.unwrap_or_default());

    let result = if let Some(dur) = timeout_duration {
        tokio::time::timeout(dur, peer.call_tool(params))
            .await
            .map_err(|_| anyhow::anyhow!("Tool call timed out after {}ms", dur.as_millis()))?
            .context("Tool call failed")
    } else {
        peer.call_tool(params).await.context("Tool call failed")
    };

    let result = match result {
        Ok(result) => result,
        Err(error) => {
            if should_mark_runtime_disconnected(&error) {
                let _ = mark_runtime_disconnected(app_handle, state, server_id, &error).await;
            }
            return Err(error);
        }
    };

    let content: Vec<serde_json::Value> = result
        .content
        .into_iter()
        .map(|c| serde_json::to_value(&c).unwrap_or(serde_json::Value::Null))
        .collect();

    Ok(CallToolResult {
        content,
        is_error: result.is_error,
    })
}

fn spawn_runtime_monitor(
    app_handle: AppHandle,
    server_id: i64,
    connection_id: Uuid,
    peer: rmcp::service::Peer<rmcp::RoleClient>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if peer.is_transport_closed() {
                let error = anyhow::anyhow!("Transport closed");
                let _ = mark_runtime_disconnected_if_current(
                    &app_handle,
                    server_id,
                    connection_id,
                    &error,
                )
                .await;
                break;
            }
        }
    })
}

fn should_mark_runtime_disconnected(error: &anyhow::Error) -> bool {
    let message = format!("{error:#}").to_ascii_lowercase();
    message.contains("transport closed")
        || message.contains("connection closed")
        || message.contains("channel closed")
        || message.contains("broken pipe")
        || message.contains("pipe has been ended")
        || message.contains("pipe is being closed")
}

async fn mark_runtime_disconnected(
    app_handle: &AppHandle,
    state: &AppState,
    server_id: i64,
    error: &anyhow::Error,
) -> anyhow::Result<()> {
    let mut removed = state.clients.lock().await.remove(&server_id);
    if let Some(holder) = removed.as_mut() {
        holder.abort_keep_alive_task();
    }

    runtime_state::update_runtime(state, server_id, |runtime| {
        runtime.connected = false;
        runtime.connecting = false;
        runtime.error = Some(format!("{error:#}"));
        runtime.discovered_at = None;
    })?;
    runtime_state::emit_runtime(app_handle, state, server_id)
}

async fn mark_runtime_disconnected_if_current(
    app_handle: &AppHandle,
    server_id: i64,
    connection_id: Uuid,
    error: &anyhow::Error,
) -> anyhow::Result<()> {
    let state = app_handle.state::<AppState>();
    let removed = {
        let mut clients = state.clients.lock().await;
        if clients
            .get(&server_id)
            .is_some_and(|holder| holder.connection_id == connection_id)
        {
            clients.remove(&server_id)
        } else {
            None
        }
    };

    let Some(mut holder) = removed else {
        return Ok(());
    };
    holder.abort_keep_alive_task();

    runtime_state::update_runtime(&state, server_id, |runtime| {
        runtime.connected = false;
        runtime.connecting = false;
        runtime.error = Some(format!("{error:#}"));
        runtime.discovered_at = None;
    })?;
    runtime_state::emit_runtime(app_handle, &state, server_id)
}
