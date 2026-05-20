use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use axum::{
    extract::DefaultBodyLimit,
    http::{header, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    Router,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use tauri::{AppHandle, Emitter};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

use crate::db::mcp_group;
use crate::gateway::handler::{GlobalHandler, GroupHandler};
use crate::state::AppState;

/// State for the running gateway server.
pub struct GatewayState {
    pub shutdown_token: CancellationToken,
    pub server_handle: JoinHandle<()>,
    pub port: u16,
}

/// Gateway status returned to the frontend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStatus {
    pub running: bool,
    pub port: Option<u16>,
    pub error: Option<String>,
}

/// Start the gateway server. Returns GatewayState on success.
pub async fn start_gateway(
    app_handle: &AppHandle,
    state: &AppState,
) -> anyhow::Result<GatewayState> {
    let settings = state.settings.read().await;
    let port = settings.port;
    let auth_enabled = settings.auth_enabled;
    let auth_token = settings.auth_token.clone();
    drop(settings);

    let groups = {
        let db = state
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to lock database"))?;
        mcp_group::list(&db)?
    };

    let cancellation_token = CancellationToken::new();

    let mut router = Router::new();

    let global_app_handle = app_handle.clone();
    let global_config = StreamableHttpServerConfig::default()
        .with_sse_keep_alive(None)
        .with_stateful_mode(true)
        .with_json_response(true)
        .with_cancellation_token(cancellation_token.child_token());
    let global_service: StreamableHttpService<GlobalHandler, LocalSessionManager> =
        StreamableHttpService::new(
            move || {
                Ok(GlobalHandler {
                    app_handle: global_app_handle.clone(),
                })
            },
            Arc::new(LocalSessionManager::default()),
            global_config,
        );
    // Register both /mcp and /mcp/ so clients that normalize base URLs with
    // a trailing slash can still reach the global endpoint, while group routes
    // under /mcp/{group} remain handled by their more specific nested routes.
    router = router.route_service("/mcp", global_service.clone());
    router = router.route_service("/mcp/", global_service);

    for group in &groups {
        let group_name = group.name.clone();
        let app_handle_clone = app_handle.clone();

        let config = StreamableHttpServerConfig::default()
            .with_sse_keep_alive(None)
            .with_stateful_mode(true)
            .with_json_response(true)
            .with_cancellation_token(cancellation_token.child_token());

        let service: StreamableHttpService<GroupHandler, LocalSessionManager> =
            StreamableHttpService::new(
                move || {
                    Ok(GroupHandler {
                        app_handle: app_handle_clone.clone(),
                        group_name: group_name.clone(),
                    })
                },
                Arc::new(LocalSessionManager::default()),
                config,
            );

        let path = format!("/mcp/{}", urlencoding::encode(&group.name));
        router = router.nest_service(path.as_str(), service);
    }

    // Auth middleware (before CORS for fail-fast)
    let router = router
        .layer(middleware::from_fn(
            move |req: axum::http::Request<axum::body::Body>, next: Next| {
                let auth_enabled = auth_enabled;
                let auth_token = auth_token.clone();
                async move {
                    if !auth_enabled {
                        return next.run(req).await;
                    }
                    let auth_header = req
                        .headers()
                        .get(header::AUTHORIZATION)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.strip_prefix("Bearer "));
                    match auth_header {
                        Some(token) if token == auth_token => next.run(req).await,
                        _ => StatusCode::UNAUTHORIZED.into_response(),
                    }
                }
            },
        ))
        .layer(CorsLayer::permissive())
        // Limit request body to 10 MiB to prevent abuse
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Set SO_REUSEADDR so we can re-bind quickly after restart (avoids TIME_WAIT issues)
    let socket = tokio::net::TcpSocket::new_v4().context("Failed to create socket")?;
    socket
        .set_reuseaddr(true)
        .context("Failed to set SO_REUSEADDR")?;
    socket
        .bind(addr)
        .with_context(|| format!("Failed to bind port {port}"))?;

    let tcp_listener = socket
        .listen(1024)
        .with_context(|| format!("Failed to listen on port {port}"))?;

    let actual_port = tcp_listener
        .local_addr()
        .context("Failed to get local address")?
        .port();

    let handle = app_handle.clone();
    let ct = cancellation_token.clone();
    let server_handle = tokio::spawn(async move {
        let result = axum::serve(tcp_listener, router)
            .with_graceful_shutdown(async move { ct.cancelled().await })
            .await;
        if let Err(e) = result {
            eprintln!("Gateway server error: {e}");
            let _ = handle.emit("gateway:error", format!("Gateway server error: {e}"));
        }
    });

    Ok(GatewayState {
        shutdown_token: cancellation_token,
        server_handle,
        port: actual_port,
    })
}

/// Stop the gateway server and wait for it to fully release the port.
pub async fn stop_gateway(gateway: GatewayState) {
    gateway.shutdown_token.cancel();
    // Wait for the server task to finish so the port is fully released
    let _ = gateway.server_handle.await;
}

/// Restart the gateway: stop the old one, start a new one.
pub async fn restart_gateway(
    app_handle: &AppHandle,
    state: &AppState,
) -> anyhow::Result<GatewayState> {
    let old_gateway = state.gateway.write().await.take();
    if let Some(old) = old_gateway {
        stop_gateway(old).await;
    }
    start_gateway(app_handle, state).await
}
