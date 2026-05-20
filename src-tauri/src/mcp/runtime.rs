use anyhow::Context;
use rmcp::service::RunningService;
use rmcp::RoleClient;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use uuid::Uuid;

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
    pub connection_id: Uuid,
    pub client: Option<RunningService<RoleClient, ()>>,
    pub keep_alive_handle: Option<JoinHandle<()>>,
    pub monitor_handle: Option<JoinHandle<()>>,
}

impl McpClientHolder {
    pub const fn new(client: RunningService<RoleClient, ()>, connection_id: Uuid) -> Self {
        Self {
            connection_id,
            client: Some(client),
            keep_alive_handle: None,
            monitor_handle: None,
        }
    }

    pub fn abort_background_tasks(&mut self) {
        if let Some(handle) = self.keep_alive_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.monitor_handle.take() {
            handle.abort();
        }
    }

    pub fn abort_keep_alive_task(&mut self) {
        if let Some(handle) = self.keep_alive_handle.take() {
            handle.abort();
        }
    }

    pub async fn close(&mut self) -> anyhow::Result<()> {
        self.abort_background_tasks();
        if let Some(client) = self.client.as_mut() {
            client.close().await.context("Failed to close MCP client")?;
        }
        self.client = None;
        Ok(())
    }
}
