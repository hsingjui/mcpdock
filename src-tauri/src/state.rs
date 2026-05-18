use std::collections::HashMap;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::db::app_settings::AppSettings;
use crate::gateway::server::GatewayState;
use crate::mcp::runtime::{McpClientHolder, McpServerRuntime};

pub struct AppState {
    pub db: Mutex<Connection>,
    pub runtimes: Mutex<HashMap<i64, McpServerRuntime>>,
    pub clients: tokio::sync::Mutex<HashMap<i64, McpClientHolder>>,
    pub settings: tokio::sync::RwLock<AppSettings>,
    pub gateway: tokio::sync::RwLock<Option<GatewayState>>,
    pub gateway_error: tokio::sync::RwLock<Option<String>>,
}

impl AppState {
    pub fn new(db: Mutex<Connection>, settings: AppSettings) -> Self {
        Self {
            db,
            runtimes: Mutex::new(HashMap::new()),
            clients: tokio::sync::Mutex::new(HashMap::new()),
            settings: tokio::sync::RwLock::new(settings),
            gateway: tokio::sync::RwLock::new(None),
            gateway_error: tokio::sync::RwLock::new(None),
        }
    }
}
