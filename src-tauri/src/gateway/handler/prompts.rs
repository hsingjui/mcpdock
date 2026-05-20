use rmcp::model::{GetPromptRequestParams, GetPromptResult, ListPromptsResult, Prompt};
use tauri::Manager;

use crate::gateway::handler::{find_peer_by_id, with_request_timeout, GroupHandler};
use crate::state::AppState;

pub(super) async fn list_prompts(
    handler: &GroupHandler,
) -> Result<ListPromptsResult, rmcp::ErrorData> {
    let config = handler
        .load_group_config()
        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
    let (separator, timeout) = handler.get_settings().await;
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
                    server_sel.prompts.clone(),
                    client.peer().clone(),
                ))
            })
            .collect()
    };

    // Query all upstream servers concurrently
    let mut tasks: Vec<tokio::task::JoinHandle<Result<Vec<Prompt>, rmcp::ErrorData>>> =
        Vec::with_capacity(server_peers.len());
    for (server_name, allowed_prompts, peer) in &server_peers {
        let server_name = server_name.clone();
        let allowed_prompts = allowed_prompts.clone();
        let peer = peer.clone();
        let separator = separator.clone();
        tasks.push(tokio::spawn(async move {
            let server_prompts_result = match timeout {
                Some(dur) => tokio::time::timeout(dur, peer.list_all_prompts())
                    .await
                    .map_err(|_| {
                        rmcp::ErrorData::internal_error(
                            format!(
                                "Listing prompts from server '{}' timed out after {}ms",
                                server_name,
                                dur.as_millis()
                            ),
                            None,
                        )
                    })?,
                None => peer.list_all_prompts().await,
            };
            let server_prompts = server_prompts_result.map_err(|e| {
                rmcp::ErrorData::internal_error(
                    format!("Failed to list prompts from server '{server_name}': {e}"),
                    None,
                )
            })?;

            let mut prefixed = Vec::new();
            for prompt in server_prompts {
                if let Some(ref allowed) = allowed_prompts {
                    if !allowed.contains(&prompt.name) {
                        continue;
                    }
                }
                let prefixed_name = format!("{}{}{}", server_name, separator, prompt.name);
                prefixed.push(Prompt::new(
                    prefixed_name,
                    prompt.description,
                    prompt.arguments,
                ));
            }
            Ok(prefixed)
        }));
    }

    let mut prompts = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(server_prompts)) => prompts.extend(server_prompts),
            Ok(Err(e)) => return Err(e),
            Err(e) => {
                return Err(rmcp::ErrorData::internal_error(
                    format!("Task panicked: {e}"),
                    None,
                ))
            }
        }
    }

    Ok(ListPromptsResult {
        prompts,
        next_cursor: None,
        ..Default::default()
    })
}

pub(super) async fn get_prompt(
    handler: &GroupHandler,
    request: GetPromptRequestParams,
) -> Result<GetPromptResult, rmcp::ErrorData> {
    let config = handler
        .load_group_config()
        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;
    let (separator, timeout) = handler.get_settings().await;

    let (server_name, original_name) = GroupHandler::parse_prefixed_name(&request.name, &separator)
        .ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!("Invalid prompt name format: '{}'. Expected '{{server}}{{separator}}{{prompt}}' with separator '{separator}'", request.name),
                None,
            )
        })?;

    let server_sel = config
        .servers
        .iter()
        .find(|s| s.name == server_name)
        .ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!("Server '{server_name}' not found in group"),
                None,
            )
        })?;

    if let Some(ref allowed) = server_sel.prompts {
        if !allowed.contains(&original_name) {
            return Err(rmcp::ErrorData::invalid_params(
                format!("Prompt '{original_name}' is not allowed in this group"),
                None,
            ));
        }
    }

    // Clone the peer and release the lock before awaiting.
    let peer = {
        let state = handler.app_handle.state::<AppState>();
        find_peer_by_id(&state, server_sel.server_id, &server_name).await?
    };

    let params = GetPromptRequestParams::new(original_name)
        .with_arguments(request.arguments.unwrap_or_default());

    let result =
        with_request_timeout(timeout, &server_name, "Get prompt", peer.get_prompt(params)).await?;

    Ok(result)
}
