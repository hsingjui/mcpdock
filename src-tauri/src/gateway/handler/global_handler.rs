use std::collections::HashMap;
use std::time::Duration;

use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use tauri::{AppHandle, Manager};

use super::{
    collect_partial_results, find_peer_by_id, load_settings, with_request_timeout, GroupHandler,
};
use crate::db::mcp_server;
use crate::state::AppState;

pub struct GlobalHandler {
    pub app_handle: AppHandle,
}

struct ConnectedServerPeer {
    name: String,
    peer: rmcp::service::Peer<rmcp::RoleClient>,
}

impl GlobalHandler {
    async fn get_settings(&self) -> (String, Option<Duration>) {
        load_settings(&self.app_handle).await
    }

    fn load_server_names(&self) -> anyhow::Result<HashMap<i64, (String, bool)>> {
        let state = self.app_handle.state::<AppState>();
        let db = state
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to lock database"))?;
        Ok(mcp_server::list(&db)?
            .into_iter()
            .map(|server| (server.id, (server.name, server.enabled)))
            .collect())
    }

    /// Collect all currently connected MCP servers.
    /// Skips servers that are connected but not found in the database
    /// (e.g., due to a recent deletion race) instead of failing.
    async fn list_connected_servers(&self) -> Result<Vec<ConnectedServerPeer>, rmcp::ErrorData> {
        let server_names = self
            .load_server_names()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
        let state = self.app_handle.state::<AppState>();

        let mut servers = Vec::new();
        let clients = state.clients.lock().await;
        for (server_id, holder) in &*clients {
            let Some(client) = holder.client.as_ref() else {
                continue;
            };
            // Skip connected servers not present in the database (stale entry)
            let Some((server_name, enabled)) = server_names.get(server_id).cloned() else {
                eprintln!(
                    "Warning: Connected MCP server {server_id} not found in database, skipping"
                );
                continue;
            };
            if !enabled {
                continue;
            }
            servers.push(ConnectedServerPeer {
                name: server_name,
                peer: client.peer().clone(),
            });
        }
        drop(clients);

        servers.sort_unstable_by(|left, right| left.name.cmp(&right.name));
        Ok(servers)
    }

    async fn connected_peer_by_name(
        &self,
        server_name: &str,
    ) -> Result<rmcp::service::Peer<rmcp::RoleClient>, rmcp::ErrorData> {
        let server_info = self
            .load_server_names()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
        let (server_id, enabled) = server_info
            .into_iter()
            .find_map(|(id, (name, enabled))| (name == server_name).then_some((id, enabled)))
            .ok_or_else(|| {
                rmcp::ErrorData::invalid_params(format!("Server '{server_name}' not found"), None)
            })?;

        if !enabled {
            return Err(rmcp::ErrorData::invalid_params(
                format!("Server '{server_name}' is disabled"),
                None,
            ));
        }

        let state = self.app_handle.state::<AppState>();
        find_peer_by_id(&state, server_id, server_name).await
    }
}

impl ServerHandler for GlobalHandler {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new("MCPDock", "0.1.1"))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        let (separator, timeout) = self.get_settings().await;
        let server_peers = self.list_connected_servers().await?;

        let mut tasks: Vec<tokio::task::JoinHandle<Result<Vec<Tool>, rmcp::ErrorData>>> =
            Vec::with_capacity(server_peers.len());
        for server in &server_peers {
            let server_name = server.name.clone();
            let peer = server.peer.clone();
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

                let mut prefixed = Vec::with_capacity(server_tools.len());
                for tool in server_tools {
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

        let tools = collect_partial_results(tasks).await;

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let (separator, timeout) = self.get_settings().await;
        let (server_name, original_name) =
            GroupHandler::parse_prefixed_name(&request.name, &separator).ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!(
                        "Invalid tool name format: '{}'. Expected '{{server}}{{separator}}{{tool}}' with separator '{separator}'",
                        request.name
                    ),
                    None,
                )
            })?;

        let peer = self.connected_peer_by_name(&server_name).await?;
        let params = CallToolRequestParams::new(original_name)
            .with_arguments(request.arguments.unwrap_or_default());

        let result =
            with_request_timeout(timeout, &server_name, "Tool call", peer.call_tool(params))
                .await?;

        Ok(result)
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, rmcp::ErrorData> {
        let (separator, timeout) = self.get_settings().await;
        let server_peers = self.list_connected_servers().await?;

        let mut tasks: Vec<tokio::task::JoinHandle<Result<Vec<Prompt>, rmcp::ErrorData>>> =
            Vec::with_capacity(server_peers.len());
        for server in &server_peers {
            let server_name = server.name.clone();
            let peer = server.peer.clone();
            let separator = separator.clone();
            tasks.push(tokio::spawn(async move {
                let server_prompts_result = match timeout {
                    Some(dur) => tokio::time::timeout(dur, peer.list_all_prompts())
                        .await
                        .map_err(|_| {
                            rmcp::ErrorData::internal_error(
                                format!(
                                    "Listing prompts from server '{}' timed out after {}ms",
                                    server_name,
                                    dur.as_millis()
                                ),
                                None,
                            )
                        })?,
                    None => peer.list_all_prompts().await,
                };

                let server_prompts = server_prompts_result.map_err(|e| {
                    rmcp::ErrorData::internal_error(
                        format!("Failed to list prompts from server '{server_name}': {e}"),
                        None,
                    )
                })?;

                let mut prefixed = Vec::with_capacity(server_prompts.len());
                for prompt in server_prompts {
                    let prefixed_name = format!("{}{}{}", server_name, separator, prompt.name);
                    prefixed.push(Prompt::new(
                        prefixed_name,
                        prompt.description,
                        prompt.arguments,
                    ));
                }
                Ok(prefixed)
            }));
        }

        let prompts = collect_partial_results(tasks).await;

        Ok(ListPromptsResult {
            prompts,
            next_cursor: None,
            ..Default::default()
        })
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, rmcp::ErrorData> {
        let (separator, timeout) = self.get_settings().await;
        let (server_name, original_name) =
            GroupHandler::parse_prefixed_name(&request.name, &separator).ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!(
                        "Invalid prompt name format: '{}'. Expected '{{server}}{{separator}}{{prompt}}' with separator '{separator}'",
                        request.name
                    ),
                    None,
                )
            })?;

        let peer = self.connected_peer_by_name(&server_name).await?;
        let params = GetPromptRequestParams::new(original_name)
            .with_arguments(request.arguments.unwrap_or_default());

        let result =
            with_request_timeout(timeout, &server_name, "Get prompt", peer.get_prompt(params))
                .await?;

        Ok(result)
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        let (_, timeout) = self.get_settings().await;
        let server_peers = self.list_connected_servers().await?;

        let mut tasks: Vec<tokio::task::JoinHandle<Result<Vec<_>, rmcp::ErrorData>>> =
            Vec::with_capacity(server_peers.len());
        for server in &server_peers {
            let server_name = server.name.clone();
            let peer = server.peer.clone();
            tasks.push(tokio::spawn(async move {
                let server_resources_result = match timeout {
                    Some(dur) => tokio::time::timeout(dur, peer.list_all_resources())
                        .await
                        .map_err(|_| {
                            rmcp::ErrorData::internal_error(
                                format!(
                                    "Listing resources from server '{}' timed out after {}ms",
                                    server_name,
                                    dur.as_millis()
                                ),
                                None,
                            )
                        })?,
                    None => peer.list_all_resources().await,
                };
                server_resources_result.map_err(|e| {
                    rmcp::ErrorData::internal_error(
                        format!("Failed to list resources from server '{server_name}': {e}"),
                        None,
                    )
                })
            }));
        }

        let resources = collect_partial_results(tasks).await;

        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            ..Default::default()
        })
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, rmcp::ErrorData> {
        let (_, timeout) = self.get_settings().await;
        let server_peers = self.list_connected_servers().await?;

        let mut tasks: Vec<tokio::task::JoinHandle<Result<Vec<_>, rmcp::ErrorData>>> =
            Vec::with_capacity(server_peers.len());
        for server in &server_peers {
            let server_name = server.name.clone();
            let peer = server.peer.clone();
            tasks.push(tokio::spawn(async move {
                let server_templates_result = match timeout {
                    Some(dur) => tokio::time::timeout(dur, peer.list_all_resource_templates())
                        .await
                        .map_err(|_| {
                            rmcp::ErrorData::internal_error(
                                format!(
                                    "Listing resource templates from server '{}' timed out after {}ms",
                                    server_name,
                                    dur.as_millis()
                                ),
                                None,
                            )
                        })?,
                    None => peer.list_all_resource_templates().await,
                };
                server_templates_result.map_err(|e| {
                    rmcp::ErrorData::internal_error(
                        format!(
                            "Failed to list resource templates from server '{server_name}': {e}"
                        ),
                        None,
                    )
                })
            }));
        }

        let templates = collect_partial_results(tasks).await;

        Ok(ListResourceTemplatesResult {
            resource_templates: templates,
            next_cursor: None,
            ..Default::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        let (_, timeout) = self.get_settings().await;
        let server_peers = self.list_connected_servers().await?;
        let uri = &request.uri;

        for server in &server_peers {
            let result = match timeout {
                Some(dur) => tokio::time::timeout(dur, server.peer.read_resource(request.clone()))
                    .await
                    .map_err(|_| {
                        rmcp::ErrorData::internal_error(
                            format!(
                                "Reading resource '{uri}' from server '{}' timed out after {}ms",
                                server.name,
                                dur.as_millis()
                            ),
                            None,
                        )
                    })?,
                None => server.peer.read_resource(request.clone()).await,
            };
            if let Ok(result) = result {
                return Ok(result);
            }
        }

        Err(rmcp::ErrorData::invalid_params(
            format!("Resource '{uri}' not found in any connected server"),
            None,
        ))
    }
}
