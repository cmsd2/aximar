use std::sync::Arc;

use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use tokio_util::sync::CancellationToken;

use aximar_core::maxima::backend::{Backend, BackendKind, ContainerEngine};
use aximar_core::maxima::output::{MultiOutputSink, OutputSink};

use aximar_mcp::server::AximarMcpServer;

use crate::state::AppState;
use crate::tauri_output::{emit_app_log, TauriOutputSink};

use crate::commands::notebook::emit_notebook_state;
use aximar_core::commands::CommandEffect;

/// Start the embedded MCP streamable HTTP server.
///
/// The server shares the same Maxima session, catalog, and notebook state as the
/// Tauri app, so MCP-triggered changes appear live in the GUI and vice versa.
pub async fn start_mcp_server(state: AppState, listen_address: String, ct: CancellationToken) {

    // Build composite output sink: Tauri frontend + MCP capture
    let tauri_sink: Arc<dyn OutputSink> =
        Arc::new(TauriOutputSink::new(state.app_handle.clone()));
    let capture_sink: Arc<dyn OutputSink> = state.capture_sink.clone();
    let process_sink: Arc<dyn OutputSink> =
        Arc::new(MultiOutputSink::new(vec![tauri_sink, capture_sink]));

    // Build notebook-change callback that emits a Tauri event
    let app_handle = state.app_handle.clone();
    let notebook_for_cb = state.notebook.clone();
    let on_notebook_change: Arc<dyn Fn(CommandEffect) + Send + Sync> = Arc::new(move |effect| {
        let app_handle = app_handle.clone();
        let notebook = notebook_for_cb.clone();
        // Spawn a task because the callback is called synchronously but we need
        // to lock the async mutex
        tokio::spawn(async move {
            if let Ok(guard) = app_handle.try_lock() {
                if let Some(ref handle) = *guard {
                    let nb = notebook.lock().await;
                    emit_notebook_state(handle, &nb, &effect);
                }
            }
        });
    });

    // Default backend config — the MCP server uses these when auto-starting a
    // session.  When the GUI starts the session first the backend is already
    // configured and this is not used.
    let backend = Backend::from_config(
        BackendKind::default(),
        "",
        "",
        ContainerEngine::default(),
    );
    let maxima_path: Option<String> = None;
    let eval_timeout: u64 = 30;

    let server = AximarMcpServer::new_connected(
        state.session.clone(),
        state.catalog.clone(),
        state.docs.clone(),
        state.notebook.clone(),
        state.capture_sink.clone(),
        process_sink,
        state.server_log.clone(),
        backend,
        maxima_path,
        eval_timeout,
        on_notebook_change,
    );

    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig {
            stateful_mode: true,
            cancellation_token: ct.child_token(),
            ..Default::default()
        },
    );

    let router = axum::Router::new().nest_service("/mcp", service);

    let addr = listen_address;
    let app_handle_for_log = state.app_handle.clone();
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            tracing::info!("MCP HTTP server listening on http://{addr}/mcp");
            emit_app_log(&app_handle_for_log, &state.app_log, "info", &format!("MCP server listening on http://{addr}/mcp"), "mcp");
            if let Err(e) = axum::serve(listener, router)
                .with_graceful_shutdown(async move { ct.cancelled_owned().await })
                .await
            {
                tracing::error!("MCP HTTP server error: {e}");
                emit_app_log(&app_handle_for_log, &state.app_log, "error", &format!("MCP server error: {e}"), "mcp");
            }
            emit_app_log(&app_handle_for_log, &state.app_log, "info", "MCP server stopped", "mcp");
        }
        Err(e) => {
            tracing::error!("Failed to bind MCP HTTP server to {addr}: {e}");
            emit_app_log(&app_handle_for_log, &state.app_log, "error", &format!("MCP server failed to bind to {addr}: {e}"), "mcp");
        }
    }
}
