use anyhow::Context;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Default value for `low_resource_mode_enabled` when missing from JSON/DB.
const fn default_true() -> bool {
    true
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub port: u16,
    pub proxy_url: String,
    pub auth_enabled: bool,
    pub auth_token: String,
    pub request_timeout_enabled: bool,
    pub request_timeout_ms: u64,
    pub keep_alive_enabled: bool,
    pub keep_alive_interval_ms: u64,
    pub gateway_separator: String,
    pub locale: String,
    pub auto_start_enabled: bool,
    pub auto_start_hidden: bool,
    pub theme: String,
    /// When enabled, closing the main window destroys the WebView to release
    /// UI resources while the background gateway keeps running. Defaults to
    /// `true` for new and existing installations.
    #[serde(default = "default_true")]
    pub low_resource_mode_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            port: 3100,
            proxy_url: String::new(),
            auth_enabled: false,
            auth_token: String::new(),
            request_timeout_enabled: true,
            request_timeout_ms: 60_000,
            keep_alive_enabled: false,
            keep_alive_interval_ms: 60_000,
            gateway_separator: "__".to_string(),
            locale: "auto".to_string(),
            auto_start_enabled: false,
            auto_start_hidden: false,
            theme: "system".to_string(),
            low_resource_mode_enabled: true,
        }
    }
}

#[allow(dead_code)]
const SETTINGS_KEYS: &[&str] = &[
    "port",
    "proxy_url",
    "auth_enabled",
    "auth_token",
    "request_timeout_enabled",
    "request_timeout_ms",
    "keep_alive_enabled",
    "keep_alive_interval_ms",
    "gateway_separator",
    "locale",
    "auto_start_enabled",
    "auto_start_hidden",
    "theme",
    "low_resource_mode_enabled",
];

fn get_string(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        rusqlite::params![key],
        |row| row.get(0),
    )
    .ok()
}

fn upsert(conn: &Connection, key: &str, value: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )
    .with_context(|| format!("Failed to upsert setting '{key}'"))?;
    Ok(())
}

pub fn get_all(conn: &Connection) -> AppSettings {
    AppSettings {
        port: get_string(conn, "port")
            .and_then(|v| v.parse().ok())
            .unwrap_or(3100),
        proxy_url: get_string(conn, "proxy_url").unwrap_or_default(),
        auth_enabled: get_string(conn, "auth_enabled")
            .and_then(|v| v.parse().ok())
            .unwrap_or(false),
        auth_token: get_string(conn, "auth_token").unwrap_or_default(),
        request_timeout_enabled: get_string(conn, "request_timeout_enabled")
            .and_then(|v| v.parse().ok())
            .unwrap_or(true),
        request_timeout_ms: get_string(conn, "request_timeout_ms")
            .and_then(|v| v.parse().ok())
            .unwrap_or(60_000),
        keep_alive_enabled: get_string(conn, "keep_alive_enabled")
            .and_then(|v| v.parse().ok())
            .unwrap_or(false),
        keep_alive_interval_ms: get_string(conn, "keep_alive_interval_ms")
            .and_then(|v| v.parse().ok())
            .unwrap_or(60_000),
        gateway_separator: get_string(conn, "gateway_separator")
            .unwrap_or_else(|| "__".to_string()),
        locale: get_string(conn, "locale").unwrap_or_else(|| "auto".to_string()),
        auto_start_enabled: get_string(conn, "auto_start_enabled")
            .and_then(|v| v.parse().ok())
            .unwrap_or(false),
        auto_start_hidden: get_string(conn, "auto_start_hidden")
            .and_then(|v| v.parse().ok())
            .unwrap_or(false),
        theme: get_string(conn, "theme").unwrap_or_else(|| "system".to_string()),
        low_resource_mode_enabled: get_string(conn, "low_resource_mode_enabled")
            .and_then(|v| v.parse().ok())
            .unwrap_or(true),
    }
}

pub fn update_all(conn: &Connection, settings: &AppSettings) -> anyhow::Result<AppSettings> {
    let pairs: Vec<(&str, String)> = vec![
        ("port", settings.port.to_string()),
        ("proxy_url", settings.proxy_url.clone()),
        ("auth_enabled", settings.auth_enabled.to_string()),
        ("auth_token", settings.auth_token.clone()),
        (
            "request_timeout_enabled",
            settings.request_timeout_enabled.to_string(),
        ),
        (
            "request_timeout_ms",
            settings.request_timeout_ms.to_string(),
        ),
        (
            "keep_alive_enabled",
            settings.keep_alive_enabled.to_string(),
        ),
        (
            "keep_alive_interval_ms",
            settings.keep_alive_interval_ms.to_string(),
        ),
        ("gateway_separator", settings.gateway_separator.clone()),
        ("locale", settings.locale.clone()),
        (
            "auto_start_enabled",
            settings.auto_start_enabled.to_string(),
        ),
        ("auto_start_hidden", settings.auto_start_hidden.to_string()),
        ("theme", settings.theme.clone()),
        (
            "low_resource_mode_enabled",
            settings.low_resource_mode_enabled.to_string(),
        ),
    ];

    for (key, value) in &pairs {
        upsert(conn, key, value)?;
    }

    // Ensure any missing keys are populated with defaults
    let current = get_all(conn);
    Ok(current)
}

#[allow(dead_code)]
pub fn ensure_defaults(conn: &Connection) -> anyhow::Result<()> {
    let _unused = get_all(conn);
    let defaults = AppSettings::default();

    for &key in SETTINGS_KEYS {
        if get_string(conn, key).is_none() {
            let value = match key {
                "port" => defaults.port.to_string(),
                "proxy_url" => defaults.proxy_url.clone(),
                "auth_enabled" => defaults.auth_enabled.to_string(),
                "auth_token" => defaults.auth_token.clone(),
                "request_timeout_enabled" => defaults.request_timeout_enabled.to_string(),
                "request_timeout_ms" => defaults.request_timeout_ms.to_string(),
                "keep_alive_enabled" => defaults.keep_alive_enabled.to_string(),
                "keep_alive_interval_ms" => defaults.keep_alive_interval_ms.to_string(),
                "gateway_separator" => defaults.gateway_separator.clone(),
                "locale" => defaults.locale.clone(),
                "auto_start_enabled" => defaults.auto_start_enabled.to_string(),
                "auto_start_hidden" => defaults.auto_start_hidden.to_string(),
                "theme" => defaults.theme.clone(),
                "low_resource_mode_enabled" => defaults.low_resource_mode_enabled.to_string(),
                _ => String::new(),
            };
            upsert(conn, key, &value)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_db() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    #[test]
    fn default_has_low_resource_mode_enabled_true() {
        let s = AppSettings::default();
        assert!(s.low_resource_mode_enabled);
    }

    #[test]
    fn missing_db_key_defaults_to_true() {
        let conn = mem_db();
        conn.execute_batch("CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .unwrap();
        let s = get_all(&conn);
        assert!(s.low_resource_mode_enabled);
    }

    #[test]
    fn db_round_trip_persists_low_resource_mode() {
        let conn = mem_db();
        conn.execute_batch("CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)")
            .unwrap();
        let s = AppSettings {
            low_resource_mode_enabled: false,
            ..AppSettings::default()
        };
        let updated = update_all(&conn, &s).unwrap();
        assert!(!updated.low_resource_mode_enabled);
        let reread = get_all(&conn);
        assert!(!reread.low_resource_mode_enabled);
    }

    #[test]
    fn old_json_without_field_defaults_true() {
        // Simulates importing a backup that predates the new field.
        let json = r#"{
            "port": 3100,
            "proxyUrl": "",
            "authEnabled": false,
            "authToken": "",
            "requestTimeoutEnabled": true,
            "requestTimeoutMs": 60000,
            "keepAliveEnabled": false,
            "keepAliveIntervalMs": 60000,
            "gatewaySeparator": "__",
            "locale": "auto",
            "autoStartEnabled": false,
            "autoStartHidden": false,
            "theme": "system"
        }"#;
        let s: AppSettings = serde_json::from_str(json).unwrap();
        assert!(s.low_resource_mode_enabled);
    }

    #[test]
    fn ensure_defaults_populates_low_resource_mode() {
        let conn = mem_db();
        conn.execute_batch(
            "CREATE TABLE app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .unwrap();
        ensure_defaults(&conn).unwrap();
        let s = get_all(&conn);
        assert!(s.low_resource_mode_enabled);
    }
}
