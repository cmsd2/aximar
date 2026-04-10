use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use aximar_core::maxima::backend::{Backend, BackendKind, ContainerEngine};
use aximar_core::maxima::output::{MultiOutputSink, OutputSink};

use aximar_mcp::server::{AximarMcpServer, ProcessSinkFactory};

use crate::state::AppState;
use crate::tauri_output::{emit_app_log, TauriOutputSink};

use crate::commands::notebook::emit_notebook_state;
use crate::commands::session::build_session_status_callback;
use aximar_core::commands::CommandEffect;
use tauri::Emitter;

/// Start the embedded MCP streamable HTTP server.
///
/// The server shares the same notebook registry, catalog, and state as the
/// Tauri app, so MCP-triggered changes appear live in the GUI and vice versa.
pub async fn start_mcp_server(state: AppState, listen_address: String, token: String, ct: CancellationToken) {

    // Build process sink factory: creates MultiOutputSink(TauriSink + CaptureSink)
    // for each notebook when spawning a Maxima session.
    let app_handle_for_factory = state.app_handle.clone();
    let process_sink_factory: ProcessSinkFactory = Arc::new(move |notebook_id, capture_sink| {
        let tauri_sink: Arc<dyn OutputSink> = Arc::new(TauriOutputSink::with_notebook_id(
            app_handle_for_factory.clone(),
            notebook_id.to_string(),
        ));
        let capture: Arc<dyn OutputSink> = capture_sink.clone();
        Arc::new(MultiOutputSink::new(vec![tauri_sink, capture]))
    });

    // Build notebook-change callback that emits a Tauri event
    let app_handle = state.app_handle.clone();
    let registry_for_cb = state.registry.clone();
    let on_notebook_change: Arc<dyn Fn(&str, CommandEffect) + Send + Sync> =
        Arc::new(move |notebook_id: &str, effect: CommandEffect| {
            let app_handle = app_handle.clone();
            let registry = registry_for_cb.clone();
            let nb_id = notebook_id.to_string();
            tokio::spawn(async move {
                let notebook = {
                    let reg = registry.lock().await;
                    match reg.get(&nb_id) {
                        Ok(ctx) => ctx.notebook.clone(),
                        Err(_) => return,
                    }
                };
                if let Ok(guard) = app_handle.try_lock() {
                    if let Some(ref handle) = *guard {
                        let nb = notebook.lock().await;
                        emit_notebook_state(handle, &nb_id, &nb, &effect);
                    }
                }
            });
        });

    // Build notebook lifecycle callback that emits Tauri events for
    // MCP-initiated create/close/switch so the frontend stays in sync.
    let app_handle_for_lifecycle = state.app_handle.clone();
    let on_notebook_lifecycle: Arc<dyn Fn(&str, &str) + Send + Sync> =
        Arc::new(move |notebook_id: &str, event_type: &str| {
            let app_handle = app_handle_for_lifecycle.clone();
            let nb_id = notebook_id.to_string();
            let evt = event_type.to_string();
            tokio::spawn(async move {
                if let Ok(guard) = app_handle.try_lock() {
                    if let Some(ref handle) = *guard {
                        let _ = handle.emit("notebook-lifecycle", serde_json::json!({
                            "notebook_id": nb_id,
                            "event": evt,
                        }));
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

    // Build session-status callback so MCP-initiated session starts are
    // reflected in the GUI's session status indicator.
    let on_session_status = build_session_status_callback(&state);

    let server = AximarMcpServer::new_connected(
        state.registry.clone(),
        state.catalog.clone(),
        state.docs.clone(),
        state.packages.clone(),
        backend,
        maxima_path,
        eval_timeout,
        process_sink_factory,
        on_notebook_change,
        on_notebook_lifecycle,
        on_session_status,
    );

    let addr = listen_address;
    let app_handle_for_log = state.app_handle.clone();
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            tracing::info!("MCP HTTP server listening on http://{addr}/mcp");
            emit_app_log(&app_handle_for_log, &state.app_log, "info", &format!("MCP server listening on http://{addr}/mcp"), "mcp");
            if let Err(e) = aximar_mcp::http::serve_mcp_http(server, listener, Some(token), ct).await {
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
