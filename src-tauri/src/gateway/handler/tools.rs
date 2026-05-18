use rmcp::model::{CallToolRequestParams, CallToolResult, ListToolsResult, Tool};
use tauri::Manager;

use crate::gateway::handler::GroupHandler;
use crate::state::AppState;

pub(super) async fn list_tools(handler: &GroupHandler) -> Result<ListToolsResult, rmcp::ErrorData> {
    let config = handler
        .load_group_config()
        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
    let (separator, timeout) = handler.get_settings().await;
    let state = handler.app_handle.state::<AppState>();

    // Clone all needed peers under a short lock, then release before awaiting.
    let server_peers: Vec<(
        String,
        Option<Vec<String>>,
        rmcp::service::Peer<rmcp::RoleClient>,
    )> = {
        let clients = state.clients.lock().await;
        config
            .servers
            .iter()
            .filter_map(|server_sel| {
                let holder = clients.get(&server_sel.server_id)?;
                let client = holder.client.as_ref()?;
                Some((
                    server_sel.name.clone(),
                    server_sel.tools.clone(),
                    client.peer().clone(),
                ))
            })
            .collect()
    };

    // Query all upstream servers concurrently
    let mut tasks: Vec<tokio::task::JoinHandle<Result<Vec<Tool>, rmcp::ErrorData>>> =
        Vec::with_capacity(server_peers.len());
    for (server_name, allowed_tools, peer) in &server_peers {
        let server_name = server_name.clone();
        let allowed_tools = allowed_tools.clone();
        let peer = peer.clone();
        let separator = separator.clone();
        tasks.push(tokio::spawn(async move {
            let server_tools_result = match timeout {
                Some(dur) => tokio::time::timeout(dur, peer.list_all_tools())
                    .await
                    .map_err(|_| {
                        rmcp::ErrorData::internal_error(
                            format!(
                                "Listing tools from server '{}' timed out after {}ms",
                                server_name,
                                dur.as_millis()
                            ),
                            None,
                        )
                    })?,
                None => peer.list_all_tools().await,
            };
            let server_tools = server_tools_result.map_err(|e| {
                rmcp::ErrorData::internal_error(
                    format!("Failed to list tools from server '{server_name}': {e}"),
                    None,
                )
            })?;

            let mut prefixed = Vec::new();
            for tool in server_tools {
                if let Some(ref allowed) = allowed_tools {
                    if !allowed.contains(&tool.name.to_string()) {
                        continue;
                    }
                }
                let prefixed_name = format!("{}{}{}", server_name, separator, tool.name);
                let prefixed_tool =
                    Tool::new_with_raw(prefixed_name, tool.description, tool.input_schema);
                let prefixed_tool = match tool.output_schema {
                    Some(schema) => prefixed_tool.with_raw_output_schema(schema),
                    None => prefixed_tool,
                };
                let prefixed_tool = match tool.annotations {
                    Some(ann) => prefixed_tool.with_annotations(ann),
                    None => prefixed_tool,
                };
                prefixed.push(prefixed_tool);
            }
            Ok(prefixed)
        }));
    }

    let mut tools = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(server_tools)) => tools.extend(server_tools),
            Ok(Err(e)) => return Err(e),
            Err(e) => {
                return Err(rmcp::ErrorData::internal_error(
                    format!("Task panicked: {e}"),
                    None,
                ))
            }
        }
    }

    Ok(ListToolsResult {
        tools,
        next_cursor: None,
        ..Default::default()
    })
}

pub(super) async fn call_tool(
    handler: &GroupHandler,
    request: CallToolRequestParams,
) -> Result<CallToolResult, rmcp::ErrorData> {
    let config = handler
        .load_group_config()
        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
    let (separator, timeout) = handler.get_settings().await;

    let (server_name, original_name) = GroupHandler::parse_prefixed_name(&request.name, &separator)
        .ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!("Invalid tool name format: '{}'. Expected '{{server}}{{separator}}{{tool}}' with separator '{separator}'", request.name),
                None,
            )
        })?;

    let server_sel = config
        .servers
        .iter()
        .find(|s| s.name == server_name)
        .ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!("Server '{server_name}' not found in group"),
                None,
            )
        })?;

    if let Some(ref allowed) = server_sel.tools {
        if !allowed.contains(&original_name) {
            return Err(rmcp::ErrorData::invalid_params(
                format!("Tool '{original_name}' is not allowed in this group"),
                None,
            ));
        }
    }

    let state = handler.app_handle.state::<AppState>();

    // Clone the peer and release the lock before awaiting.
    let peer = {
        let clients = state.clients.lock().await;
        let holder = clients.get(&server_sel.server_id).ok_or_else(|| {
            rmcp::ErrorData::internal_error(
                format!("MCP server '{server_name}' is not connected"),
                None,
            )
        })?;
        let client = holder.client.as_ref().ok_or_else(|| {
            rmcp::ErrorData::internal_error(
                format!("MCP server '{server_name}' client is unavailable"),
                None,
            )
        })?;
        client.peer().clone()
    };

    let params = CallToolRequestParams::new(original_name)
        .with_arguments(request.arguments.unwrap_or_default());

    let result = match timeout {
        Some(dur) => tokio::time::timeout(dur, peer.call_tool(params))
            .await
            .map_err(|_| {
                rmcp::ErrorData::internal_error(
                    format!(
                        "Tool call to server '{server_name}' timed out after {}ms",
                        dur.as_millis()
                    ),
                    None,
                )
            })?
            .map_err(|e| {
                rmcp::ErrorData::internal_error(
                    format!("Tool call failed on server '{server_name}': {e}"),
                    None,
                )
            })?,
        None => peer.call_tool(params).await.map_err(|e| {
            rmcp::ErrorData::internal_error(
                format!("Tool call failed on server '{server_name}': {e}"),
                None,
            )
        })?,
    };

    Ok(result)
}
