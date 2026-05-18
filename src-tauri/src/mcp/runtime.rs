use anyhow::Context;
use rmcp::service::RunningService;
use rmcp::RoleClient;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerRuntime {
    pub server_id: i64,
    pub connected: bool,
    pub connecting: bool,
    pub error: Option<String>,
    pub discovered_at: Option<i64>,
}

impl McpServerRuntime {
    pub const fn new(server_id: i64) -> Self {
        Self {
            server_id,
            connected: false,
            connecting: false,
            error: None,
            discovered_at: None,
        }
    }
}

pub struct DiscoveryResult {
    pub tools: Vec<serde_json::Value>,
    pub resources: Vec<serde_json::Value>,
    pub resource_templates: Vec<serde_json::Value>,
    pub prompts: Vec<serde_json::Value>,
}

pub struct McpClientHolder {
    pub client: Option<RunningService<RoleClient, ()>>,
    pub keep_alive_handle: Option<JoinHandle<()>>,
}

impl McpClientHolder {
    pub const fn new(client: RunningService<RoleClient, ()>) -> Self {
        Self {
            client: Some(client),
            keep_alive_handle: None,
        }
    }

    pub async fn close(&mut self) -> anyhow::Result<()> {
        // Cancel the keep-alive task first
        if let Some(handle) = self.keep_alive_handle.take() {
            handle.abort();
        }
        if let Some(client) = self.client.as_mut() {
            client.close().await.context("Failed to close MCP client")?;
        }
        self.client = None;
        Ok(())
    }
}
