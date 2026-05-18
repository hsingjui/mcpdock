mod prompts;
mod resources;
mod tools;

use std::time::Duration;

use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use tauri::{AppHandle, Manager};

use crate::db::mcp_group;
use crate::state::AppState;

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
        let state = self.app_handle.state::<AppState>();
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
