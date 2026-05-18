use std::collections::HashMap;

use anyhow::Context;
use tauri::{AppHandle, Emitter};

use crate::mcp::runtime::McpServerRuntime;
use crate::state::AppState;

pub(super) const RUNTIME_CHANGED_EVENT: &str = "mcp:runtime-changed";

pub(super) fn set_runtime(
    state: &AppState,
    runtime: McpServerRuntime,
) -> anyhow::Result<McpServerRuntime> {
    let mut runtimes = state
        .runtimes
        .lock()
        .map_err(|_| anyhow::anyhow!("Failed to lock runtimes"))?;
    runtimes.insert(runtime.server_id, runtime.clone());
    Ok(runtime)
}

pub(super) fn update_runtime<F>(
    state: &AppState,
    server_id: i64,
    update: F,
) -> anyhow::Result<McpServerRuntime>
where
    F: FnOnce(&mut McpServerRuntime),
{
    let mut runtimes = state
        .runtimes
        .lock()
        .map_err(|_| anyhow::anyhow!("Failed to lock runtimes"))?;

    let runtime = runtimes
        .entry(server_id)
        .or_insert_with(|| McpServerRuntime::new(server_id));
    update(runtime);
    Ok(runtime.clone())
}

pub(super) fn emit_runtime(
    app_handle: &AppHandle,
    state: &AppState,
    server_id: i64,
) -> anyhow::Result<()> {
    let runtime = get_runtime(state, server_id)?;
    app_handle
        .emit(RUNTIME_CHANGED_EVENT, runtime)
        .context("Failed to emit runtime event")
}

pub fn get_runtime(state: &AppState, server_id: i64) -> anyhow::Result<McpServerRuntime> {
    let runtimes = state
        .runtimes
        .lock()
        .map_err(|_| anyhow::anyhow!("Failed to lock runtimes"))?;

    Ok(runtimes
        .get(&server_id)
        .cloned()
        .unwrap_or_else(|| McpServerRuntime::new(server_id)))
}

pub fn list_runtimes(state: &AppState) -> anyhow::Result<HashMap<i64, McpServerRuntime>> {
    let runtimes = state
        .runtimes
        .lock()
        .map_err(|_| anyhow::anyhow!("Failed to lock runtimes"))?;

    Ok(runtimes.clone())
}
