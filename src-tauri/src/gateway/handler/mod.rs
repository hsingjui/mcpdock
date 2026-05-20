mod global_handler;
mod prompts;
mod resources;
mod tools;

use std::future::Future;
use std::time::Duration;

use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleClient, RoleServer};
use tauri::{AppHandle, Manager};

use crate::db::mcp_group;
use crate::state::AppState;

pub use global_handler::GlobalHandler;

pub(super) async fn load_settings(app_handle: &AppHandle) -> (String, Option<Duration>) {
    let state = app_handle.state::<AppState>();
    let settings = state.settings.read().await;
    (
        settings.gateway_separator.clone(),
        if settings.request_timeout_enabled {
            Some(Duration::from_millis(settings.request_timeout_ms))
        } else {
            None
        },
    )
}

/// Find a connected peer for a server by its ID.
/// Looks up the server in the active clients map and returns its peer handle.
pub(super) async fn find_peer_by_id(
    state: &AppState,
    server_id: i64,
    server_name: &str,
) -> Result<rmcp::service::Peer<RoleClient>, rmcp::ErrorData> {
    let clients = state.clients.lock().await;
    let holder = clients.get(&server_id).ok_or_else(|| {
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
    Ok(client.peer().clone())
}

/// Execute an async MCP request with optional timeout, mapping both timeout
/// and protocol errors to `rmcp::ErrorData` with descriptive messages.
pub(super) async fn with_request_timeout<T>(
    timeout: Option<Duration>,
    server_name: &str,
    operation: &str,
    future: impl Future<Output = Result<T, rmcp::service::ServiceError>>,
) -> Result<T, rmcp::ErrorData> {
    match timeout {
        Some(dur) => tokio::time::timeout(dur, future)
            .await
            .map_err(|_| {
                rmcp::ErrorData::internal_error(
                    format!(
                        "{operation} to server '{server_name}' timed out after {}ms",
                        dur.as_millis()
                    ),
                    None,
                )
            })?
            .map_err(|e| {
                rmcp::ErrorData::internal_error(
                    format!("{operation} failed on server '{server_name}': {e}"),
                    None,
                )
            }),
        None => future.await.map_err(|e| {
            rmcp::ErrorData::internal_error(
                format!("{operation} failed on server '{server_name}': {e}"),
                None,
            )
        }),
    }
}

/// Collect results from concurrent aggregation tasks, tolerating individual
/// server failures. On partial failure, logs a warning and continues with
/// results from successful servers.
pub(super) async fn collect_partial_results<T: Send + 'static>(
    tasks: Vec<tokio::task::JoinHandle<Result<Vec<T>, rmcp::ErrorData>>>,
) -> Vec<T> {
    let mut items = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(server_items)) => items.extend(server_items),
            Ok(Err(e)) => {
                eprintln!("Warning: {e}");
            }
            Err(e) => {
                eprintln!("Warning: Aggregation task panicked: {e}");
            }
        }
    }
    items
}

/// A per-group MCP server handler that aggregates tools/prompts/resources
/// from multiple upstream MCP servers, with name prefixing for disambiguation.
pub struct GroupHandler {
    pub app_handle: AppHandle,
    pub group_name: String,
}

impl GroupHandler {
    /// Load the group config from the database.
    pub(super) fn load_group_config(&self) -> anyhow::Result<mcp_group::McpGroupConfig> {
        let state = self.app_handle.state::<AppState>();
        let db = state
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to lock database"))?;
        let groups = mcp_group::list(&db)?;
        let group = groups
            .into_iter()
            .find(|g| g.name == self.group_name)
            .ok_or_else(|| anyhow::anyhow!("Group '{}' not found", self.group_name))?;
        Ok(group.config)
    }

    /// Get the separator and request timeout from settings in a single lock acquisition.
    pub(super) async fn get_settings(&self) -> (String, Option<Duration>) {
        load_settings(&self.app_handle).await
    }

    /// Parse a prefixed name like "filesystem__read_file" into (`server_name`, `original_name`).
    pub(super) fn parse_prefixed_name(prefixed: &str, separator: &str) -> Option<(String, String)> {
        let idx = prefixed.find(separator)?;
        let server_name = prefixed[..idx].to_string();
        let original_name = prefixed[idx + separator.len()..].to_string();
        if server_name.is_empty() || original_name.is_empty() {
            return None;
        }
        Some((server_name, original_name))
    }
}

impl ServerHandler for GroupHandler {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new(
            format!("MCPDock-{}", self.group_name),
            "0.1.1",
        ))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        tools::list_tools(self).await
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        tools::call_tool(self, request).await
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, rmcp::ErrorData> {
        prompts::list_prompts(self).await
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResult, rmcp::ErrorData> {
        prompts::get_prompt(self, request).await
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        resources::list_resources(self).await
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, rmcp::ErrorData> {
        resources::list_resource_templates(self).await
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        resources::read_resource(self, request).await
    }
}
