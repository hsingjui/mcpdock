use rmcp::model::{
    ListResourceTemplatesResult, ListResourcesResult, ReadResourceRequestParams, ReadResourceResult,
};
use tauri::Manager;

use crate::gateway::handler::GroupHandler;
use crate::state::AppState;

pub(super) async fn list_resources(
    handler: &GroupHandler,
) -> Result<ListResourcesResult, rmcp::ErrorData> {
    let config = handler
        .load_group_config()
        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
    let (_, timeout) = handler.get_settings().await;
    let state = handler.app_handle.state::<AppState>();

    // Clone all needed peers under a short lock, then release before awaiting.
    let server_peers: Vec<(
        String,
        Option<Vec<String>>,
        rmcp::service::Peer<rmcp::RoleClient>,
    )> = {
        let clients = state.clients.lock().await;
        config
            .servers
            .iter()
            .filter_map(|server_sel| {
                let holder = clients.get(&server_sel.server_id)?;
                let client = holder.client.as_ref()?;
                Some((
                    server_sel.name.clone(),
                    server_sel.resources.clone(),
                    client.peer().clone(),
                ))
            })
            .collect()
    };

    // Query all upstream servers concurrently
    let mut tasks: Vec<tokio::task::JoinHandle<Result<Vec<_>, rmcp::ErrorData>>> =
        Vec::with_capacity(server_peers.len());
    for (server_name, allowed_resources, peer) in &server_peers {
        let server_name = server_name.clone();
        let allowed_resources = allowed_resources.clone();
        let peer = peer.clone();
        tasks.push(tokio::spawn(async move {
            let server_resources_result = match timeout {
                Some(dur) => tokio::time::timeout(dur, peer.list_all_resources())
                    .await
                    .map_err(|_| {
                        rmcp::ErrorData::internal_error(
                            format!(
                                "Listing resources from server '{}' timed out after {}ms",
                                server_name,
                                dur.as_millis()
                            ),
                            None,
                        )
                    })?,
                None => peer.list_all_resources().await,
            };
            let server_resources = server_resources_result.map_err(|e| {
                rmcp::ErrorData::internal_error(
                    format!("Failed to list resources from server '{server_name}': {e}"),
                    None,
                )
            })?;
            Ok(server_resources
                .into_iter()
                .filter(|r| {
                    allowed_resources
                        .as_ref()
                        .is_none_or(|allowed| allowed.contains(&r.uri))
                })
                .collect())
        }));
    }

    let mut resources = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(server_resources)) => resources.extend(server_resources),
            Ok(Err(e)) => return Err(e),
            Err(e) => {
                return Err(rmcp::ErrorData::internal_error(
                    format!("Task panicked: {e}"),
                    None,
                ))
            }
        }
    }

    Ok(ListResourcesResult {
        resources,
        next_cursor: None,
        ..Default::default()
    })
}

pub(super) async fn list_resource_templates(
    handler: &GroupHandler,
) -> Result<ListResourceTemplatesResult, rmcp::ErrorData> {
    let config = handler
        .load_group_config()
        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
    let (_, timeout) = handler.get_settings().await;
    let state = handler.app_handle.state::<AppState>();

    // Clone all needed peers under a short lock, then release before awaiting.
    let server_peers: Vec<(
        String,
        Option<Vec<String>>,
        rmcp::service::Peer<rmcp::RoleClient>,
    )> = {
        let clients = state.clients.lock().await;
        config
            .servers
            .iter()
            .filter_map(|server_sel| {
                let holder = clients.get(&server_sel.server_id)?;
                let client = holder.client.as_ref()?;
                Some((
                    server_sel.name.clone(),
                    server_sel.resources.clone(),
                    client.peer().clone(),
                ))
            })
            .collect()
    };

    // Query all upstream servers concurrently
    let mut tasks: Vec<tokio::task::JoinHandle<Result<Vec<_>, rmcp::ErrorData>>> =
        Vec::with_capacity(server_peers.len());
    for (server_name, allowed_resources, peer) in &server_peers {
        let server_name = server_name.clone();
        let allowed_resources = allowed_resources.clone();
        let peer = peer.clone();
        tasks.push(tokio::spawn(async move {
            let server_templates_result = match timeout {
                Some(dur) => tokio::time::timeout(dur, peer.list_all_resource_templates())
                    .await
                    .map_err(|_| {
                        rmcp::ErrorData::internal_error(
                            format!(
                                "Listing resource templates from server '{}' timed out after {}ms",
                                server_name,
                                dur.as_millis()
                            ),
                            None,
                        )
                    })?,
                None => peer.list_all_resource_templates().await,
            };
            let server_templates = server_templates_result.map_err(|e| {
                rmcp::ErrorData::internal_error(
                    format!("Failed to list resource templates from server '{server_name}': {e}"),
                    None,
                )
            })?;
            Ok(server_templates
                .into_iter()
                .filter(|t| {
                    allowed_resources
                        .as_ref()
                        .is_none_or(|allowed| allowed.contains(&t.uri_template))
                })
                .collect())
        }));
    }

    let mut templates = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(server_templates)) => templates.extend(server_templates),
            Ok(Err(e)) => return Err(e),
            Err(e) => {
                return Err(rmcp::ErrorData::internal_error(
                    format!("Task panicked: {e}"),
                    None,
                ))
            }
        }
    }

    Ok(ListResourceTemplatesResult {
        resource_templates: templates,
        next_cursor: None,
        ..Default::default()
    })
}

pub(super) async fn read_resource(
    handler: &GroupHandler,
    request: ReadResourceRequestParams,
) -> Result<ReadResourceResult, rmcp::ErrorData> {
    let config = handler
        .load_group_config()
        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
    let (_, timeout) = handler.get_settings().await;
    let state = handler.app_handle.state::<AppState>();

    // Clone all candidate peers under a short lock, then release before awaiting.
    let uri = &request.uri;
    let candidates: Vec<(String, rmcp::service::Peer<rmcp::RoleClient>)> = {
        let clients = state.clients.lock().await;
        config
            .servers
            .iter()
            .filter(|server_sel| {
                server_sel
                    .resources
                    .as_ref()
                    .is_none_or(|allowed| allowed.contains(uri))
            })
            .filter_map(|server_sel| {
                let holder = clients.get(&server_sel.server_id)?;
                let client = holder.client.as_ref()?;
                Some((server_sel.name.clone(), client.peer().clone()))
            })
            .collect()
    };

    for (server_name, peer) in &candidates {
        let result = match timeout {
            Some(dur) => tokio::time::timeout(dur, peer.read_resource(request.clone()))
                .await
                .map_err(|_| {
                    rmcp::ErrorData::internal_error(
                        format!(
                            "Reading resource '{uri}' from server '{}' timed out after {}ms",
                            server_name,
                            dur.as_millis()
                        ),
                        None,
                    )
                })?,
            None => peer.read_resource(request.clone()).await,
        };
        if let Ok(result) = result {
            return Ok(result);
        }
    }

    Err(rmcp::ErrorData::invalid_params(
        format!("Resource '{uri}' not found in any connected server of this group"),
        None,
    ))
}
