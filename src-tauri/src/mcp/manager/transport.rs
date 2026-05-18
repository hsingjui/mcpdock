use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Context;
use http::{HeaderName, HeaderValue};
use rmcp::transport::{
    streamable_http_client::StreamableHttpClientTransportConfig, StreamableHttpClientTransport,
    TokioChildProcess,
};
use rmcp::ServiceExt;

use crate::db::mcp_server::{normalize_stdio_command, McpServerRow};
use crate::process_env::{
    build_stdio_environment, find_executable_on_path, spawn_stderr_collector,
};
use crate::state::AppState;

pub(super) async fn connect_stdio(
    server: &McpServerRow,
) -> anyhow::Result<rmcp::service::RunningService<rmcp::RoleClient, ()>> {
    let command = normalize_stdio_command(server.command.as_deref())
        .ok_or_else(|| anyhow::anyhow!("STDIO server command is required"))?;

    let args: Vec<String> = serde_json::from_str(&server.args).context("Invalid args JSON")?;
    let server_env: HashMap<String, String> =
        serde_json::from_str(&server.env).context("Invalid env JSON")?;
    let parent_env = std::env::vars().collect::<HashMap<_, _>>();
    let env = build_stdio_environment(&parent_env, &server_env);
    let resolved_command = find_executable_on_path(&command, &env).unwrap_or(command);

    let mut cmd = build_stdio_command(&resolved_command, &args);
    cmd.env_clear();
    cmd.envs(&env);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());

    let (transport, stderr) = TokioChildProcess::builder(cmd)
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn MCP child process")?;
    let stderr_buffer = spawn_stderr_collector(stderr);

    ().serve(transport).await.map_err(|error| {
        let base = anyhow::Error::new(error).context("Failed to connect stdio MCP server");
        match stderr_buffer.summary() {
            Some(summary) => base.context(format!("stdio stderr: {summary}")),
            None => base,
        }
    })
}

fn build_stdio_command(command: &str, args: &[String]) -> tokio::process::Command {
    #[cfg(target_os = "windows")]
    {
        return build_windows_stdio_command(command, args);
    }

    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args);
        cmd
    }
}

#[cfg(target_os = "windows")]
fn build_windows_stdio_command(command: &str, args: &[String]) -> tokio::process::Command {
    if windows_needs_cmd_launcher(command) {
        let mut cmd = tokio::process::Command::new("cmd.exe");
        cmd.arg("/C");
        cmd.arg(command);
        cmd.args(args);
        return cmd;
    }

    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args);
    cmd
}

#[cfg(target_os = "windows")]
fn windows_needs_cmd_launcher(command: &str) -> bool {
    use std::path::Path;

    let command_path = Path::new(command);
    let extension = command_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);

    if matches!(extension.as_deref(), Some("cmd" | "bat")) {
        return true;
    }

    if windows_looks_like_explicit_path(command_path) {
        return false;
    }

    windows_resolve_on_path(command)
        .and_then(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(str::to_ascii_lowercase)
        })
        .map(|ext| matches!(ext.as_str(), "cmd" | "bat"))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn windows_looks_like_explicit_path(path: &std::path::Path) -> bool {
    path.is_absolute()
        || path
            .parent()
            .is_some_and(|parent| !parent.as_os_str().is_empty())
        || path.has_root()
}

#[cfg(target_os = "windows")]
fn windows_resolve_on_path(command: &str) -> Option<std::path::PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let pathext = std::env::var_os("PATHEXT")
        .map(windows_split_path_ext)
        .filter(|exts| !exts.is_empty())
        .unwrap_or_else(windows_default_path_ext);

    let command_path = std::path::Path::new(command);
    let has_extension = command_path.extension().is_some();

    for dir in std::env::split_paths(&path_var) {
        if has_extension {
            let candidate = dir.join(command);
            if candidate.is_file() {
                return Some(candidate);
            }
            continue;
        }

        for ext in &pathext {
            let candidate = dir.join(format!("{command}{ext}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn windows_split_path_ext(value: std::ffi::OsString) -> Vec<String> {
    value
        .to_string_lossy()
        .split(';')
        .map(str::trim)
        .filter(|ext| !ext.is_empty())
        .map(|ext| {
            if ext.starts_with('.') {
                ext.to_string()
            } else {
                format!(".{ext}")
            }
        })
        .collect()
}

#[cfg(target_os = "windows")]
fn windows_default_path_ext() -> Vec<String> {
    [".COM", ".EXE", ".BAT", ".CMD"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

pub(super) async fn connect_streamable_http(
    state: &AppState,
    server: &McpServerRow,
) -> anyhow::Result<rmcp::service::RunningService<rmcp::RoleClient, ()>> {
    let url = server
        .url
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Streamable HTTP server url is required"))?;

    let headers: HashMap<String, String> =
        serde_json::from_str(&server.headers).context("Invalid headers JSON")?;

    let custom_headers = headers
        .into_iter()
        .map(|(key, value)| {
            let name = HeaderName::try_from(key).context("Invalid header name")?;
            let value = HeaderValue::try_from(value).context("Invalid header value")?;
            Ok((name, value))
        })
        .collect::<anyhow::Result<HashMap<HeaderName, HeaderValue>>>()?;

    let config = StreamableHttpClientTransportConfig::with_uri(url).custom_headers(custom_headers);

    let settings = state.settings.read().await;

    let mut client_builder = reqwest::Client::builder().pool_max_idle_per_host(0);

    if settings.request_timeout_enabled {
        client_builder =
            client_builder.connect_timeout(Duration::from_millis(settings.request_timeout_ms));
    }
    if settings.keep_alive_enabled {
        client_builder =
            client_builder.tcp_keepalive(Duration::from_millis(settings.keep_alive_interval_ms));
    }
    if !settings.proxy_url.is_empty() {
        let proxy = reqwest::Proxy::all(&settings.proxy_url).context("Invalid proxy URL")?;
        client_builder = client_builder.proxy(proxy);
    }

    let client = client_builder
        .build()
        .context("Failed to build HTTP client")?;
    let transport = StreamableHttpClientTransport::with_client(client, config);

    if settings.request_timeout_enabled {
        tokio::time::timeout(
            Duration::from_millis(settings.request_timeout_ms),
            ().serve(transport),
        )
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "Streamable HTTP connection timed out after {}ms",
                settings.request_timeout_ms
            )
        })?
        .context("Failed to connect streamable HTTP MCP server")
    } else {
        ().serve(transport)
            .await
            .context("Failed to connect streamable HTTP MCP server")
    }
}
