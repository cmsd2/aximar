use std::sync::Arc;
use serde::Serialize;
use tauri::{Emitter, State};

use aximar_core::error::AppError;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::{MultiOutputSink, OutputSink};
use aximar_core::maxima::process::MaximaProcess;
use aximar_core::maxima::protocol;
use aximar_core::maxima::types::SessionStatus;
use aximar_core::maxima::unicode::build_texput_init;
use aximar_core::registry::NotebookContextRef;
use crate::commands::config::{read_backend, read_eval_timeout, read_maxima_path};
use crate::state::AppState;
use crate::tauri_output::{emit_app_log, TauriOutputSink};

#[derive(Clone, Serialize)]
struct SessionStatusEvent {
    notebook_id: String,
    status: SessionStatus,
}

fn emit_session_status(state: &AppState, notebook_id: &str, status: SessionStatus) {
    if let Ok(guard) = state.app_handle.try_lock() {
        if let Some(ref handle) = *guard {
            let _ = handle.emit("session-status-changed", SessionStatusEvent {
                notebook_id: notebook_id.to_string(),
                status,
            });
        }
    }
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
    backend: Backend,
    maxima_path: Option<String>,
    eval_timeout: u64,
) -> Result<(), AppError> {
    let status = ctx.session.status();
    match status {
        SessionStatus::Ready | SessionStatus::Busy => Ok(()),
        SessionStatus::Stopped | SessionStatus::Error(_) => {
            let output_sink = build_output_sink(state, ctx);
            emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session auto-starting...", "session");
            ctx.session.begin_start().await;
            emit_session_status(state, &ctx.id, SessionStatus::Starting);

            match MaximaProcess::spawn(backend, maxima_path, output_sink).await {
                Ok(process) => {
                    ctx.session.set_ready(process).await;
                    let init = build_texput_init();
                    let mut guard = ctx.session.lock().await;
                    if let Ok(p) = guard.process_mut() {
                        let _ = protocol::evaluate(p, "__init__", &init, &state.catalog, eval_timeout).await;
                    }
                    drop(guard);
                    emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session ready", "session");
                    emit_session_status(state, &ctx.id, SessionStatus::Ready);
                    Ok(())
                }
                Err(e) => {
                    let msg = e.to_string();
                    ctx.session.set_error(msg.clone()).await;
                    emit_app_log(&state.app_handle, &state.app_log, "error", &format!("Maxima session failed: {msg}"), "session");
                    emit_session_status(state, &ctx.id, SessionStatus::Error(msg));
                    Err(e)
                }
            }
        }
        SessionStatus::Starting => {
            for _ in 0..50 {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                match ctx.session.status() {
                    SessionStatus::Ready | SessionStatus::Busy => return Ok(()),
                    SessionStatus::Error(_) | SessionStatus::Stopped => {
                        return Err(AppError::ProcessNotRunning);
                    }
                    _ => continue,
                }
            }
            Err(AppError::ProcessNotRunning)
        }
    }
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
    let output_sink = build_output_sink(&state, &ctx);

    emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session starting...", "session");
    ctx.session.begin_start().await;

    match MaximaProcess::spawn(backend, maxima_path, output_sink).await {
        Ok(process) => {
            ctx.session.set_ready(process).await;
            // Configure texput so Greek letters render correctly in TeX output
            let init = build_texput_init();
            let eval_timeout = read_eval_timeout(&app);
            let mut guard = ctx.session.lock().await;
            if let Ok(p) = guard.process_mut() {
                let _ = protocol::evaluate(p, "__init__", &init, &state.catalog, eval_timeout).await;
            }
            drop(guard);
            emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session ready", "session");
            Ok(SessionStatus::Ready)
        }
        Err(e) => {
            let msg = e.to_string();
            ctx.session.set_error(msg.clone()).await;
            emit_app_log(&state.app_handle, &state.app_log, "error", &format!("Maxima session failed: {msg}"), "session");
            Err(e)
        }
    }
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
    let output_sink = build_output_sink(&state, &ctx);

    emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session restarting...", "session");
    ctx.session.begin_start().await;

    match MaximaProcess::spawn(backend, maxima_path, output_sink).await {
        Ok(process) => {
            ctx.session.set_ready(process).await;
            // Configure texput so Greek letters render correctly in TeX output
            let init = build_texput_init();
            let eval_timeout = read_eval_timeout(&app);
            let mut guard = ctx.session.lock().await;
            if let Ok(p) = guard.process_mut() {
                let _ = protocol::evaluate(p, "__init__", &init, &state.catalog, eval_timeout).await;
            }
            drop(guard);
            emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session ready", "session");
            Ok(SessionStatus::Ready)
        }
        Err(e) => {
            let msg = e.to_string();
            ctx.session.set_error(msg.clone()).await;
            emit_app_log(&state.app_handle, &state.app_log, "error", &format!("Maxima session restart failed: {msg}"), "session");
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_session_status(
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<SessionStatus, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    Ok(ctx.session.status())
}
