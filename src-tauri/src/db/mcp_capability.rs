use anyhow::Context;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCapabilityRow {
    pub id: i64,
    pub server_id: i64,
    pub r#type: String,
    pub capability_key: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub payload: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCapability {
    pub id: i64,
    pub server_id: i64,
    pub r#type: String,
    pub capability_key: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub payload: serde_json::Value,
    pub updated_at: String,
}

impl TryFrom<McpCapabilityRow> for McpCapability {
    type Error = String;

    fn try_from(row: McpCapabilityRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            server_id: row.server_id,
            r#type: row.r#type,
            capability_key: row.capability_key,
            name: row.name,
            description: row.description,
            payload: serde_json::from_str(&row.payload)
                .map_err(|e| format!("Invalid capability payload JSON: {e}"))?,
            updated_at: row.updated_at,
        })
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpCapabilityRow> {
    Ok(McpCapabilityRow {
        id: row.get(0)?,
        server_id: row.get(1)?,
        r#type: row.get(2)?,
        capability_key: row.get(3)?,
        name: row.get(4)?,
        description: row.get(5)?,
        payload: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn row_to_api(row: McpCapabilityRow) -> anyhow::Result<McpCapability> {
    row.try_into().map_err(anyhow::Error::msg)
}

fn capability_key_for(type_name: &str, payload: &serde_json::Value) -> anyhow::Result<String> {
    match type_name {
        "tool" | "prompt" => payload
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("{type_name} payload missing name")),
        "resource" => payload
            .get("uri")
            .and_then(|value| value.as_str())
            .or_else(|| payload.get("uriTemplate").and_then(|value| value.as_str()))
            .map(str::to_string)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("resource payload missing uri or uriTemplate")),
        other => Err(anyhow::anyhow!("Unsupported capability type: {other}")),
    }
}

fn name_for(payload: &serde_json::Value) -> Option<String> {
    payload
        .get("name")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn description_for(payload: &serde_json::Value) -> Option<String> {
    payload
        .get("description")
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn list_by_server(conn: &Connection, server_id: i64) -> anyhow::Result<Vec<McpCapability>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, server_id, type, capability_key, name, description, payload, updated_at \
             FROM mcp_capabilities WHERE server_id = ?1 ORDER BY type, COALESCE(name, capability_key)",
        )
        .context("Failed to prepare query")?;

    let rows = stmt
        .query_map(rusqlite::params![server_id], map_row)
        .context("Failed to query capabilities")?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row_to_api(row.context("Failed to map capability row")?)?);
    }
    Ok(result)
}

pub fn list_all(conn: &Connection) -> anyhow::Result<Vec<McpCapability>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, server_id, type, capability_key, name, description, payload, updated_at \
             FROM mcp_capabilities ORDER BY server_id, type, COALESCE(name, capability_key)",
        )
        .context("Failed to prepare query")?;

    let rows = stmt
        .query_map([], map_row)
        .context("Failed to query capabilities")?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row_to_api(row.context("Failed to map capability row")?)?);
    }
    Ok(result)
}

pub fn replace_server_capabilities(
    conn: &Connection,
    server_id: i64,
    tools: &[serde_json::Value],
    prompts: &[serde_json::Value],
    resources: &[serde_json::Value],
    resource_templates: &[serde_json::Value],
) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM mcp_capabilities WHERE server_id = ?1",
        rusqlite::params![server_id],
    )
    .context("Failed to delete capabilities")?;

    insert_many(conn, server_id, "tool", tools)?;
    insert_many(conn, server_id, "prompt", prompts)?;
    insert_many(conn, server_id, "resource", resources)?;
    insert_many(conn, server_id, "resource", resource_templates)?;

    Ok(())
}

pub fn delete_by_server(conn: &Connection, server_id: i64) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM mcp_capabilities WHERE server_id = ?1",
        rusqlite::params![server_id],
    )
    .context("Failed to delete capabilities")?;
    Ok(())
}

fn insert_many(
    conn: &Connection,
    server_id: i64,
    r#type: &str,
    items: &[serde_json::Value],
) -> anyhow::Result<()> {
    for payload in items {
        let capability_key = capability_key_for(r#type, payload)?;
        let name = name_for(payload);
        let description = description_for(payload);
        let payload_text = serde_json::to_string(payload)
            .with_context(|| format!("Failed to serialize {type} capability"))?;

        conn.execute(
            "INSERT INTO mcp_capabilities \
             (server_id, type, capability_key, name, description, payload, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)",
            rusqlite::params![
                server_id,
                r#type,
                capability_key,
                name,
                description,
                payload_text,
            ],
        )
        .with_context(|| format!("Failed to insert {type} capability"))?;
    }

    Ok(())
}
