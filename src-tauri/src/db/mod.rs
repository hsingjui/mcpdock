use std::sync::Mutex;

use anyhow::Context;
use rusqlite::Connection;
use tauri::Manager;

pub mod app_settings;
pub mod error;
pub mod mcp_capability;
pub mod mcp_group;
pub mod mcp_server;

const DB_FILE_NAME: &str = "app.db";

pub fn init_db(app_handle: &tauri::AppHandle) -> anyhow::Result<Mutex<Connection>> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .context("Failed to resolve app data dir")?;
    let db_path = app_dir.join(DB_FILE_NAME);

    std::fs::create_dir_all(&app_dir)
        .with_context(|| format!("Failed to create app data dir: {}", app_dir.display()))?;

    eprintln!("Data directory: {}", app_dir.display());

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path.display()))?;

    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;",
    )
    .context("Failed to set pragmas")?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS mcp_servers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
            transport_type TEXT NOT NULL CHECK (transport_type IN ('stdio', 'streamable_http')),
            command TEXT,
            args TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(args) AND json_type(args) = 'array'),
            env TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(env) AND json_type(env) = 'object'),
            url TEXT,
            headers TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(headers) AND json_type(headers) = 'object'),
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (length(trim(name)) > 0),
            CHECK (transport_type != 'stdio' OR (command IS NOT NULL AND length(trim(command)) > 0)),
            CHECK (transport_type != 'streamable_http' OR (url IS NOT NULL AND length(trim(url)) > 0))
        );

        CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled ON mcp_servers(enabled);
        CREATE INDEX IF NOT EXISTS idx_mcp_servers_transport_type ON mcp_servers(transport_type);

        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS mcp_capabilities (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            server_id INTEGER NOT NULL,
            type TEXT NOT NULL CHECK (type IN ('tool', 'prompt', 'resource')),
            capability_key TEXT NOT NULL,
            name TEXT,
            description TEXT,
            payload TEXT NOT NULL CHECK (json_valid(payload)),
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES mcp_servers(id) ON DELETE CASCADE,
            UNIQUE (server_id, type, capability_key)
        );

        CREATE INDEX IF NOT EXISTS idx_mcp_capabilities_server_id ON mcp_capabilities(server_id);
        CREATE INDEX IF NOT EXISTS idx_mcp_capabilities_type ON mcp_capabilities(type);

        CREATE TABLE IF NOT EXISTS mcp_groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            config TEXT NOT NULL CHECK (json_valid(config)),
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (length(trim(name)) > 0)
        );"
    )
    .context("Failed to create tables")?;

    Ok(Mutex::new(conn))
}
