use anyhow::{ensure, Context};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpGroupServerSelection {
    pub server_id: i64,
    pub name: String,
    pub tools: Option<Vec<String>>,
    pub prompts: Option<Vec<String>>,
    pub resources: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpGroupConfig {
    pub servers: Vec<McpGroupServerSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpGroup {
    pub id: String,
    pub name: String,
    pub config: McpGroupConfig,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpGroupInput {
    pub name: String,
    pub config: McpGroupConfig,
}

fn validate_selection_names(values: Option<&Vec<String>>, field: &str) -> anyhow::Result<()> {
    if let Some(items) = values {
        for item in items {
            ensure!(!item.trim().is_empty(), "{field} contains empty item");
        }
    }
    Ok(())
}

impl McpGroupInput {
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(!self.name.trim().is_empty(), "name is required");

        for server in &self.config.servers {
            ensure!(server.server_id > 0, "serverId is required");
            ensure!(!server.name.trim().is_empty(), "server name is required");
            validate_selection_names(server.tools.as_ref(), "tools")?;
            validate_selection_names(server.prompts.as_ref(), "prompts")?;
            validate_selection_names(server.resources.as_ref(), "resources")?;
        }

        Ok(())
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, String, String, String, String)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn parse_group(row: (String, String, String, String, String)) -> anyhow::Result<McpGroup> {
    let (id, name, config, created_at, updated_at) = row;
    Ok(McpGroup {
        id,
        name,
        config: serde_json::from_str(&config).context("Invalid group config JSON")?,
        created_at,
        updated_at,
    })
}

fn row_from_conn(conn: &Connection, id: &str) -> anyhow::Result<McpGroup> {
    let row = conn
        .query_row(
            "SELECT id, name, config, created_at, updated_at FROM mcp_groups WHERE id = ?1",
            rusqlite::params![id],
            map_row,
        )
        .context("Group not found")?;

    parse_group(row)
}

pub fn list(conn: &Connection) -> anyhow::Result<Vec<McpGroup>> {
    let mut stmt = conn
        .prepare("SELECT id, name, config, created_at, updated_at FROM mcp_groups ORDER BY name")
        .context("Failed to prepare query")?;

    let rows = stmt
        .query_map([], map_row)
        .context("Failed to query groups")?;

    let mut result = Vec::new();
    for row in rows {
        result.push(parse_group(row.context("Failed to map group row")?)?);
    }
    Ok(result)
}

pub fn create(conn: &Connection, input: &McpGroupInput) -> anyhow::Result<McpGroup> {
    input.validate()?;

    let id = Uuid::new_v4().to_string();
    let config =
        serde_json::to_string(&input.config).context("Failed to serialize group config")?;

    conn.execute(
        "INSERT INTO mcp_groups (id, name, config) VALUES (?1, ?2, ?3)",
        rusqlite::params![id, input.name.trim(), config],
    )
    .map_err(|e| error::on_insert(e, "分组名称"))?;

    row_from_conn(conn, &id)
}

pub fn update(conn: &Connection, id: &str, input: &McpGroupInput) -> anyhow::Result<McpGroup> {
    input.validate()?;

    let config =
        serde_json::to_string(&input.config).context("Failed to serialize group config")?;

    conn.execute(
        "UPDATE mcp_groups SET name = ?1, config = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
        rusqlite::params![input.name.trim(), config, id],
    )
    .map_err(|e| error::on_update(e, "分组名称"))?;

    row_from_conn(conn, id)
}

pub fn delete(conn: &Connection, id: &str) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM mcp_groups WHERE id = ?1",
        rusqlite::params![id],
    )
    .context("Failed to delete group")?;
    Ok(())
}

/// Remove a server from all groups' config by server id.
/// When an MCP server is deleted, this ensures no group still references it.
pub fn remove_server_from_all_groups(conn: &Connection, server_id: i64) -> anyhow::Result<()> {
    let groups = list(conn)?;
    for group in groups {
        let original_len = group.config.servers.len();
        let filtered: Vec<&McpGroupServerSelection> = group
            .config
            .servers
            .iter()
            .filter(|s| s.server_id != server_id)
            .collect();
        if filtered.len() != original_len {
            let new_config = McpGroupConfig {
                servers: filtered.into_iter().cloned().collect(),
            };
            let config_str =
                serde_json::to_string(&new_config).context("Failed to serialize group config")?;
            conn.execute(
                "UPDATE mcp_groups SET config = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                rusqlite::params![config_str, group.id],
            )
            .context("Failed to update group config after server removal")?;
        }
    }
    Ok(())
}
