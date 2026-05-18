use std::time::Duration;

use anyhow::Context;
use rmcp::RoleClient;

use crate::mcp::runtime::DiscoveryResult;
use crate::state::AppState;

pub(super) async fn discover_capabilities(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
    state: &AppState,
) -> anyhow::Result<DiscoveryResult> {
    let peer = client.peer().clone();
    discover_capabilities_from_peer(&peer, state).await
}

pub(super) async fn discover_capabilities_from_peer(
    peer: &rmcp::service::Peer<RoleClient>,
    state: &AppState,
) -> anyhow::Result<DiscoveryResult> {
    let settings = state.settings.read().await;
    let timeout = if settings.request_timeout_enabled {
        Some(Duration::from_millis(settings.request_timeout_ms))
    } else {
        None
    };
    drop(settings);

    // Discover tools, resources, templates, and prompts concurrently.
    // Tools are required; the other three are optional capabilities —
    // some MCP servers (e.g. context7) only support tools and respond with
    // "Method not found" (-32601). Fall back to empty lists for those.
    let (tools_result, resources, resource_templates, prompts) = tokio::join!(
        // Required: tools failure is fatal.
        async {
            if let Some(dur) = timeout {
                tokio::time::timeout(dur, peer.list_all_tools())
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!("Listing tools timed out after {}ms", dur.as_millis())
                    })?
                    .context("Failed to list tools")
            } else {
                peer.list_all_tools().await.context("Failed to list tools")
            }
        },
        // Optional: fall back to empty list on any failure.
        async {
            if let Some(dur) = timeout {
                match tokio::time::timeout(dur, peer.list_all_resources()).await {
                    Ok(Ok(v)) => v,
                    _ => Vec::new(),
                }
            } else {
                peer.list_all_resources().await.ok().unwrap_or_default()
            }
        },
        async {
            if let Some(dur) = timeout {
                match tokio::time::timeout(dur, peer.list_all_resource_templates()).await {
                    Ok(Ok(v)) => v,
                    _ => Vec::new(),
                }
            } else {
                peer.list_all_resource_templates()
                    .await
                    .ok()
                    .unwrap_or_default()
            }
        },
        async {
            if let Some(dur) = timeout {
                match tokio::time::timeout(dur, peer.list_all_prompts()).await {
                    Ok(Ok(v)) => v,
                    _ => Vec::new(),
                }
            } else {
                peer.list_all_prompts().await.ok().unwrap_or_default()
            }
        },
    );

    let tools = tools_result?;

    let tools = serialize_items(tools)?;
    let resources = serialize_items(resources)?;
    let resource_templates = serialize_items(resource_templates)?;
    let prompts = serialize_items(prompts)?;

    Ok(DiscoveryResult {
        tools,
        resources,
        resource_templates,
        prompts,
    })
}

fn serialize_items<T: serde::Serialize>(items: Vec<T>) -> anyhow::Result<Vec<serde_json::Value>> {
    items
        .into_iter()
        .map(|item| serde_json::to_value(item).context("Serialization failed"))
        .collect()
}
