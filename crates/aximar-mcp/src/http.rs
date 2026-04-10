//! Shared HTTP transport for the MCP server.
//!
//! Both the standalone `aximar-mcp` binary and the Tauri GUI app use this
//! module to serve MCP over Streamable HTTP.  Callers are responsible for
//! constructing the [`AximarMcpServer`], binding the [`TcpListener`], and
//! providing a [`CancellationToken`] for graceful shutdown.

use std::sync::Arc;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use tokio_util::sync::CancellationToken;

use crate::server::AximarMcpServer;

/// Serve the MCP server over HTTP.
///
/// - `server` — a fully configured [`AximarMcpServer`] (standalone or connected)
/// - `listener` — a bound [`TcpListener`] (caller controls address/port)
/// - `token` — bearer token for authentication; `None` disables auth
/// - `ct` — cancellation token for graceful shutdown
pub async fn serve_mcp_http(
    server: AximarMcpServer,
    listener: tokio::net::TcpListener,
    token: Option<String>,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig {
            stateful_mode: true,
            cancellation_token: ct.child_token(),
            ..Default::default()
        },
    );

    let router = if let Some(ref token) = token {
        let expected = format!("Bearer {token}");
        axum::Router::new()
            .nest_service("/mcp", service)
            .layer(axum::middleware::from_fn(move |req: Request, next: Next| {
                let expected = expected.clone();
                async move {
                    let auth = req
                        .headers()
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|v| v.to_str().ok());
                    match auth {
                        Some(v) if v == expected => Ok(next.run(req).await),
                        _ => Err(StatusCode::UNAUTHORIZED),
                    }
                }
            }))
    } else {
        axum::Router::new().nest_service("/mcp", service)
    };

    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;

    Ok(())
}
