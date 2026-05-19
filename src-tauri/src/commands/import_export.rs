use std::fs;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};

use crate::db::{app_settings, mcp_group, mcp_server};
use crate::gateway::server::GatewayStatus;
use crate::mcp::manager;
use crate::state::AppState;

// ── Export structures ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerExport {
    pub name: String,
    pub enabled: bool,
    pub transport_type: String,
    pub command: Option<String>,
    pub args: String,
    pub env: String,
    pub url: Option<String>,
    pub headers: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupServerExport {
    pub server_name: String,
    pub tools: Option<Vec<String>>,
    pub prompts: Option<Vec<String>>,
    pub resources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupConfigExport {
    pub servers: Vec<GroupServerExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupExport {
    pub name: String,
    pub config: GroupConfigExport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
    pub version: u32,
    pub exported_at: String,
    pub mcp_servers: Vec<McpServerExport>,
    pub groups: Vec<GroupExport>,
    pub settings: app_settings::AppSettings,
}

// ── Import structures ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerImport {
    pub name: String,
    pub enabled: Option<bool>,
    pub transport_type: String,
    pub command: Option<String>,
    pub args: Option<String>,
    pub env: Option<String>,
    pub url: Option<String>,
    pub headers: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupServerImport {
    pub server_name: String,
    pub tools: Option<Vec<String>>,
    pub prompts: Option<Vec<String>>,
    pub resources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupConfigImport {
    pub servers: Vec<GroupServerImport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupImport {
    pub name: String,
    pub config: GroupConfigImport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPayload {
    pub version: u32,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerImport>,
    #[serde(default)]
    pub groups: Vec<GroupImport>,
    #[serde(default)]
    pub settings: Option<app_settings::AppSettings>,
}

// ── Result structures ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCounts {
    pub imported: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub servers: ImportCounts,
    pub groups: ImportCounts,
    pub settings_updated: bool,
}

// ── Commands ──

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn export_all_data(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let db = state
        .db
        .lock()
        .map_err(|_| "Failed to lock database".to_string())?;

    let servers = mcp_server::list(&db).map_err(|e| e.to_string())?;
    let groups = mcp_group::list(&db).map_err(|e| e.to_string())?;
    let settings = app_settings::get_all(&db);

    // Build an id→name lookup for resolving group server references
    let id_to_name: std::collections::HashMap<i64, String> =
        servers.iter().map(|s| (s.id, s.name.clone())).collect();

    let mcp_servers_export: Vec<McpServerExport> = servers
        .iter()
        .map(|s| McpServerExport {
            name: s.name.clone(),
            enabled: s.enabled,
            transport_type: s.transport_type.clone(),
            command: s.command.clone(),
            args: s.args.clone(),
            env: s.env.clone(),
            url: s.url.clone(),
            headers: s.headers.clone(),
        })
        .collect();

    let groups_export: Vec<GroupExport> = groups
        .iter()
        .map(|g| {
            let servers_out: Vec<GroupServerExport> = g
                .config
                .servers
                .iter()
                .map(|s| {
                    let server_name = id_to_name
                        .get(&s.server_id)
                        .cloned()
                        .unwrap_or_else(|| format!("unknown_{}", s.server_id));
                    GroupServerExport {
                        server_name,
                        tools: s.tools.clone(),
                        prompts: s.prompts.clone(),
                        resources: s.resources.clone(),
                    }
                })
                .collect();
            GroupExport {
                name: g.name.clone(),
                config: GroupConfigExport {
                    servers: servers_out,
                },
            }
        })
        .collect();

    let payload = ExportPayload {
        version: 1,
        exported_at: chrono::Utc::now().to_rfc3339(),
        mcp_servers: mcp_servers_export,
        groups: groups_export,
        settings,
    };

    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;

    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub async fn import_all_data(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportResult, String> {
    let json = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {e}"))?;

    let payload: ImportPayload =
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse JSON: {e}"))?;

    if payload.version != 1 {
        return Err(format!(
            "Unsupported version: {}, only version 1 is supported",
            payload.version
        ));
    }

    // ── Synchronous DB work inside a block with transaction ──
    let (result, new_settings, imported_enabled_server_ids) = {
        let mut db = state
            .db
            .lock()
            .map_err(|_| "Failed to lock database".to_string())?;
        let tx = db
            .transaction()
            .map_err(|e| format!("Failed to start transaction: {e}"))?;

        // 1. Import MCP servers (skip if name conflict)
        let mut servers_imported: usize = 0;
        let mut servers_skipped: usize = 0;
        let mut imported_enabled_server_ids = Vec::new();
        let mut name_to_id: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();

        let existing = mcp_server::list(&tx).map_err(|e| e.to_string())?;
        let mut existing_names: std::collections::HashSet<String> =
            existing.iter().map(|s| s.name.clone()).collect();

        for srv in &payload.mcp_servers {
            if existing_names.contains(&srv.name) {
                // Name conflict — map existing or newly imported id for group resolution
                if let Some(server_id) = name_to_id.get(&srv.name) {
                    name_to_id.insert(srv.name.clone(), *server_id);
                } else if let Some(existing_srv) = existing.iter().find(|s| s.name == srv.name) {
                    name_to_id.insert(srv.name.clone(), existing_srv.id);
                }
                servers_skipped += 1;
                continue;
            }

            let input = mcp_server::McpServerInput {
                name: srv.name.clone(),
                enabled: srv.enabled,
                transport_type: srv.transport_type.clone(),
                command: srv.command.clone(),
                args: srv.args.clone(),
                env: srv.env.clone(),
                url: srv.url.clone(),
                headers: srv.headers.clone(),
            };

            let row = mcp_server::create(&tx, &input)
                .map_err(|e| format!("Failed to import server '{}': {e}", srv.name))?;
            if row.enabled {
                imported_enabled_server_ids.push(row.id);
            }
            name_to_id.insert(srv.name.clone(), row.id);
            existing_names.insert(srv.name.clone());
            servers_imported += 1;
        }

        // 2. Import groups (skip if name conflict, map serverName→serverId)
        let mut groups_imported: usize = 0;
        let mut groups_skipped: usize = 0;

        let existing_groups = mcp_group::list(&tx).map_err(|e| e.to_string())?;
        let mut existing_group_names: std::collections::HashSet<String> =
            existing_groups.iter().map(|g| g.name.clone()).collect();

        for grp in &payload.groups {
            if existing_group_names.contains(&grp.name) {
                groups_skipped += 1;
                continue;
            }

            let server_selections: Vec<mcp_group::McpGroupServerSelection> = grp
                .config
                .servers
                .iter()
                .filter_map(|s| {
                    let server_id = name_to_id.get(&s.server_name)?;
                    Some(mcp_group::McpGroupServerSelection {
                        server_id: *server_id,
                        name: s.server_name.clone(),
                        tools: s.tools.clone(),
                        prompts: s.prompts.clone(),
                        resources: s.resources.clone(),
                    })
                })
                .collect();

            let input = mcp_group::McpGroupInput {
                name: grp.name.clone(),
                config: mcp_group::McpGroupConfig {
                    servers: server_selections,
                },
            };

            mcp_group::create(&tx, &input)
                .map_err(|e| format!("Failed to import group '{}': {e}", grp.name))?;
            existing_group_names.insert(grp.name.clone());
            groups_imported += 1;
        }

        // 3. Import settings (overwrite)
        let settings_updated = if let Some(settings) = &payload.settings {
            app_settings::update_all(&tx, settings)
                .map_err(|e| format!("Failed to import settings: {e}"))?;
            true
        } else {
            false
        };

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

        let new_settings = settings_updated.then(|| {
            payload
                .settings
                .clone()
                .expect("settings_updated is true but payload.settings is None")
        });

        let result = ImportResult {
            servers: ImportCounts {
                imported: servers_imported,
                skipped: servers_skipped,
            },
            groups: ImportCounts {
                imported: groups_imported,
                skipped: groups_skipped,
            },
            settings_updated,
        };

        (result, new_settings, imported_enabled_server_ids)
    }; // db lock dropped here

    // Update in-memory settings cache
    if let Some(settings) = new_settings {
        {
            let mut guard = state.settings.write().await;
            *guard = settings;
        }

        // Restart gateway (settings may affect gateway behavior)
        match crate::gateway::server::restart_gateway(&app_handle, &state).await {
            Ok(gateway_state) => {
                let port = gateway_state.port;
                *state.gateway.write().await = Some(gateway_state);
                *state.gateway_error.write().await = None;
                let _ = app_handle.emit(
                    "gateway:status-changed",
                    GatewayStatus {
                        running: true,
                        port: Some(port),
                        error: None,
                    },
                );
            }
            Err(e) => {
                eprintln!("Failed to restart gateway after import: {e}");
                *state.gateway.write().await = None;
                *state.gateway_error.write().await = Some(e.to_string());
                let _ = app_handle.emit(
                    "gateway:status-changed",
                    GatewayStatus {
                        running: false,
                        port: None,
                        error: Some(e.to_string()),
                    },
                );
                let _ = app_handle.emit("gateway:error", e.to_string());
            }
        }
    }

    if !imported_enabled_server_ids.is_empty() {
        let app_handle = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_handle.state::<AppState>();
            for server_id in imported_enabled_server_ids {
                let _ = manager::connect(&app_handle, &state, server_id).await;
            }
        });
    }

    Ok(result)
}
