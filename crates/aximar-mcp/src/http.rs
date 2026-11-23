//! Shared HTTP transport for the MCP server.
//!
//! Both the standalone `aximar-mcp` binary and the Tauri GUI app use this
//! module to serve MCP over Streamable HTTP.  Callers are responsible for
//! constructing the server, binding the [`TcpListener`], and providing a
//! [`CancellationToken`] for graceful shutdown.

use std::sync::Arc;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use rmcp::handler::server::ServerHandler;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use tokio_util::sync::CancellationToken;

/// rmcp's config for this server. On loopback, rmcp accepts only loopback
/// `Host` headers, which stops DNS rebinding. A server on another address
/// was bound there on purpose (`--address`, or the app's listen address), so
/// it accepts any `Host` and relies on the bearer token.
fn server_config(loopback: bool, ct: CancellationToken) -> StreamableHttpServerConfig {
    let config = StreamableHttpServerConfig::default()
        .with_stateful_mode(true)
        .with_cancellation_token(ct);
    if loopback {
        config
    } else {
        config.disable_allowed_hosts()
    }
}

/// Serve an MCP server over HTTP.
///
/// - `server` — a fully configured MCP server implementing [`ServerHandler`]
/// - `listener` — a bound [`TcpListener`] (caller controls address/port)
/// - `token` — bearer token for authentication; `None` disables auth
/// - `ct` — cancellation token for graceful shutdown
pub async fn serve_mcp_http<S>(
    server: S,
    listener: tokio::net::TcpListener,
    token: Option<String>,
    ct: CancellationToken,
) -> anyhow::Result<()>
where
    S: ServerHandler + Clone + Send + Sync + 'static,
{
    let loopback = listener.local_addr()?.ip().is_loopback();
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        server_config(loopback, ct.child_token()),
    );

    let router = if let Some(ref token) = token {
        let expected = format!("Bearer {token}");
        axum::Router::new()
            .nest_service("/mcp", service)
            .layer(axum::middleware::from_fn(
                move |req: Request, next: Next| {
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
                },
            ))
    } else {
        axum::Router::new().nest_service("/mcp", service)
    };

    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_servers_accept_only_loopback_hosts() {
        let config = server_config(true, CancellationToken::new());
        assert!(config.stateful_mode);
        assert!(config.allowed_hosts.iter().any(|h| h == "localhost"));
        assert!(config
            .allowed_hosts
            .iter()
            .all(|h| ["localhost", "127.0.0.1", "::1"].contains(&h.as_str())));
    }

    #[test]
    fn servers_on_other_addresses_accept_any_host() {
        let config = server_config(false, CancellationToken::new());
        assert!(config.stateful_mode);
        assert!(config.allowed_hosts.is_empty());
    }
}
