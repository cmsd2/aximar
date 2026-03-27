use std::sync::Arc;
use serde::Serialize;
use tauri::{Emitter, State};

use aximar_core::error::AppError;
use aximar_core::maxima::output::{MultiOutputSink, OutputSink};
use aximar_core::maxima::types::SessionStatus;
use aximar_core::registry::NotebookContextRef;
use aximar_core::session_ops::{self, SessionStatusCallback};
use crate::commands::config::{read_backend, read_eval_timeout, read_maxima_path};
use crate::state::AppState;
use crate::tauri_output::{emit_app_log, TauriOutputSink};

#[derive(Clone, Serialize)]
struct SessionStatusEvent {
    notebook_id: String,
    status: SessionStatus,
}

/// Build a [`SessionStatusCallback`] that emits Tauri events and app logs.
pub fn build_session_status_callback(state: &AppState) -> SessionStatusCallback {
    let app_handle = state.app_handle.clone();
    let app_log = state.app_log.clone();
    Arc::new(move |notebook_id: &str, status: SessionStatus| {
        // Emit session-status-changed event for the frontend
        if let Ok(guard) = app_handle.try_lock() {
            if let Some(ref handle) = *guard {
                let _ = handle.emit(
                    "session-status-changed",
                    SessionStatusEvent {
                        notebook_id: notebook_id.to_string(),
                        status: status.clone(),
                    },
                );
            }
        }
        // Emit app log
        let msg = match &status {
            SessionStatus::Starting => "Maxima session starting...",
            SessionStatus::Ready => "Maxima session ready",
            SessionStatus::Error(e) => {
                emit_app_log(
                    &app_handle,
                    &app_log,
                    "error",
                    &format!("Maxima session failed: {e}"),
                    "session",
                );
                return;
            }
            _ => return,
        };
        emit_app_log(&app_handle, &app_log, "info", msg, "session");
    })
}

/// Build a composite output sink that feeds both the Tauri frontend and the MCP
/// capture sink so that both GUI and MCP see all Maxima I/O.
fn build_output_sink(state: &AppState, ctx: &NotebookContextRef) -> Arc<dyn OutputSink> {
    let tauri_sink: Arc<dyn OutputSink> =
        Arc::new(TauriOutputSink::with_notebook_id(
            state.app_handle.clone(),
            ctx.id.clone(),
        ));
    let capture_sink: Arc<dyn OutputSink> = ctx.capture_sink.clone();
    Arc::new(MultiOutputSink::new(vec![tauri_sink, capture_sink]))
}

/// Resolve notebook context: if notebook_id is provided, use it; otherwise use the active notebook.
async fn resolve_context(
    state: &AppState,
    notebook_id: Option<String>,
) -> Result<NotebookContextRef, AppError> {
    let reg = state.registry.lock().await;
    reg.resolve(notebook_id.as_deref())
        .map_err(|e| AppError::CommunicationError(e))
}

/// Ensure the Maxima session for a notebook is running. If the session is
/// stopped or in an error state, spawn a new process. If it's already ready
/// or busy, this is a no-op. If it's starting, wait for it.
pub async fn ensure_session(
    state: &AppState,
    ctx: &NotebookContextRef,
    backend: aximar_core::maxima::backend::Backend,
    maxima_path: Option<String>,
    eval_timeout: u64,
) -> Result<(), AppError> {
    let on_status = build_session_status_callback(state);
    session_ops::ensure_session(
        ctx,
        backend,
        maxima_path,
        |ctx| build_output_sink(state, ctx),
        &state.catalog,
        eval_timeout,
        Some(&on_status),
    )
    .await
}

#[tauri::command]
pub async fn start_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<SessionStatus, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let maxima_path = read_maxima_path(&app);
    let backend = read_backend(&app);
    let eval_timeout = read_eval_timeout(&app);
    let output_sink = build_output_sink(&state, &ctx);
    let on_status = build_session_status_callback(&state);

    ctx.session.begin_start().await;
    on_status(&ctx.id, SessionStatus::Starting);

    session_ops::spawn_and_init_session(
        &ctx,
        backend,
        maxima_path,
        output_sink,
        &state.catalog,
        eval_timeout,
        Some(&on_status),
    )
    .await?;

    Ok(SessionStatus::Ready)
}

#[tauri::command]
pub async fn stop_session(
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<SessionStatus, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    ctx.session.stop().await?;
    emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session stopped", "session");
    Ok(SessionStatus::Stopped)
}

#[tauri::command]
pub async fn restart_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<SessionStatus, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let maxima_path = read_maxima_path(&app);
    let backend = read_backend(&app);
    let eval_timeout = read_eval_timeout(&app);
    let output_sink = build_output_sink(&state, &ctx);
    let on_status = build_session_status_callback(&state);

    // begin_start kills any existing process via into_starting()
    ctx.session.begin_start().await;
    on_status(&ctx.id, SessionStatus::Starting);

    session_ops::spawn_and_init_session(
        &ctx,
        backend,
        maxima_path,
        output_sink,
        &state.catalog,
        eval_timeout,
        Some(&on_status),
    )
    .await?;

    Ok(SessionStatus::Ready)
}

#[tauri::command]
pub async fn get_session_status(
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<SessionStatus, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    Ok(ctx.session.status())
}
