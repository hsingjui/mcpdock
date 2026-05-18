use anyhow::{bail, ensure, Context};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerRow {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub transport_type: String,
    pub command: Option<String>,
    pub args: String,
    pub env: String,
    pub url: Option<String>,
    pub headers: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerInput {
    pub name: String,
    pub enabled: Option<bool>,
    pub transport_type: String,
    pub command: Option<String>,
    pub args: Option<String>,
    pub env: Option<String>,
    pub url: Option<String>,
    pub headers: Option<String>,
}

fn normalize_optional(value: Option<&str>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

pub fn normalize_stdio_command(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    if trimmed.is_empty() {
        return None;
    }

    let unquoted = match trimmed.as_bytes() {
        [first, middle @ .., last]
            if !middle.is_empty()
                && ((*first == b'"' && *last == b'"') || (*first == b'\'' && *last == b'\'')) =>
        {
            std::str::from_utf8(middle).ok()?.trim()
        }
        _ => trimmed,
    };

    if unquoted.is_empty() {
        None
    } else {
        Some(unquoted.to_string())
    }
}

fn validate_json_field(value: &str, expected: &str, field: &str) -> anyhow::Result<()> {
    let parsed: serde_json::Value =
        serde_json::from_str(value).with_context(|| format!("Invalid {field} JSON"))?;

    let valid = match expected {
        "array" => parsed.is_array(),
        "object" => parsed.is_object(),
        _ => false,
    };

    ensure!(valid, "{field} must be a JSON {expected}");
    Ok(())
}

impl McpServerInput {
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(!self.name.trim().is_empty(), "name is required");

        match self.transport_type.as_str() {
            "stdio" => {
                ensure!(
                    normalize_stdio_command(self.command.as_deref()).is_some(),
                    "command is required for stdio transport"
                );
            }
            "streamable_http" => {
                ensure!(
                    normalize_optional(self.url.as_deref()).is_some(),
                    "url is required for streamable_http transport"
                );
            }
            other => bail!("Unsupported transport type: {other}"),
        }

        validate_json_field(self.args.as_deref().unwrap_or("[]"), "array", "args")?;
        validate_json_field(self.env.as_deref().unwrap_or("{}"), "object", "env")?;
        validate_json_field(self.headers.as_deref().unwrap_or("{}"), "object", "headers")?;

        Ok(())
    }
}

fn row_from_conn(conn: &Connection, id: i64) -> anyhow::Result<McpServerRow> {
    conn.query_row(
        "SELECT id, name, enabled, transport_type, command, args, env, url, headers, created_at, updated_at FROM mcp_servers WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            Ok(McpServerRow {
                id: row.get(0)?,
                name: row.get(1)?,
                enabled: row.get::<_, i32>(2)? != 0,
                transport_type: row.get(3)?,
                command: row.get(4)?,
                args: row.get(5)?,
                env: row.get(6)?,
                url: row.get(7)?,
                headers: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    )
    .context("MCP server not found")
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpServerRow> {
    Ok(McpServerRow {
        id: row.get(0)?,
        name: row.get(1)?,
        enabled: row.get::<_, i32>(2)? != 0,
        transport_type: row.get(3)?,
        command: row.get(4)?,
        args: row.get(5)?,
        env: row.get(6)?,
        url: row.get(7)?,
        headers: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub fn list(conn: &Connection) -> anyhow::Result<Vec<McpServerRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, enabled, transport_type, command, args, env, url, headers, created_at, updated_at FROM mcp_servers ORDER BY id",
        )
        .context("Failed to prepare query")?;

    let rows = stmt
        .query_map([], map_row)
        .context("Failed to query MCP servers")?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.context("Failed to map MCP server row")?);
    }
    Ok(result)
}

pub fn list_enabled(conn: &Connection) -> anyhow::Result<Vec<McpServerRow>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, enabled, transport_type, command, args, env, url, headers, created_at, updated_at FROM mcp_servers WHERE enabled = 1 ORDER BY id",
        )
        .context("Failed to prepare query")?;

    let rows = stmt
        .query_map([], map_row)
        .context("Failed to query enabled MCP servers")?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.context("Failed to map MCP server row")?);
    }
    Ok(result)
}

pub fn create(conn: &Connection, input: &McpServerInput) -> anyhow::Result<McpServerRow> {
    input.validate()?;

    let enabled = i32::from(input.enabled.unwrap_or(true));
    let args = input.args.as_deref().unwrap_or("[]");
    let env = input.env.as_deref().unwrap_or("{}");
    let headers = input.headers.as_deref().unwrap_or("{}");
    let command = normalize_stdio_command(input.command.as_deref());
    let url = normalize_optional(input.url.as_deref());

    conn.execute(
        "INSERT INTO mcp_servers (name, enabled, transport_type, command, args, env, url, headers) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            input.name.trim(),
            enabled,
            input.transport_type,
            command,
            args,
            env,
            url,
            headers,
        ],
    )
    .map_err(|e| error::on_insert(e, "MCP 服务器名称"))?;

    let id = conn.last_insert_rowid();
    row_from_conn(conn, id)
}

pub fn update(conn: &Connection, id: i64, input: &McpServerInput) -> anyhow::Result<McpServerRow> {
    input.validate()?;

    let enabled = i32::from(input.enabled.unwrap_or(true));
    let args = input.args.as_deref().unwrap_or("[]");
    let env = input.env.as_deref().unwrap_or("{}");
    let headers = input.headers.as_deref().unwrap_or("{}");
    let command = normalize_stdio_command(input.command.as_deref());
    let url = normalize_optional(input.url.as_deref());

    conn.execute(
        "UPDATE mcp_servers SET name = ?1, enabled = ?2, transport_type = ?3, command = ?4, args = ?5, env = ?6, url = ?7, headers = ?8, updated_at = CURRENT_TIMESTAMP WHERE id = ?9",
        rusqlite::params![
            input.name.trim(),
            enabled,
            input.transport_type,
            command,
            args,
            env,
            url,
            headers,
            id,
        ],
    )
    .map_err(|e| error::on_update(e, "MCP 服务器名称"))?;

    row_from_conn(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM mcp_servers WHERE id = ?1",
        rusqlite::params![id],
    )
    .context("Failed to delete MCP server")?;
    Ok(())
}

pub fn toggle(conn: &Connection, id: i64) -> anyhow::Result<McpServerRow> {
    conn.execute(
        "UPDATE mcp_servers SET enabled = 1 - enabled, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
        rusqlite::params![id],
    )
    .context("Failed to toggle MCP server")?;

    row_from_conn(conn, id)
}

pub fn get(conn: &Connection, id: i64) -> anyhow::Result<McpServerRow> {
    row_from_conn(conn, id)
}
