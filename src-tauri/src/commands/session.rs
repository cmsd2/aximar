use std::sync::Arc;
use tauri::State;

use aximar_core::error::AppError;
use aximar_core::maxima::output::{MultiOutputSink, OutputSink};
use aximar_core::maxima::process::MaximaProcess;
use aximar_core::maxima::types::SessionStatus;
use crate::commands::config::{read_backend, read_maxima_path};
use crate::state::AppState;
use crate::tauri_output::{emit_app_log, TauriOutputSink};

/// Build a composite output sink that feeds both the Tauri frontend and the MCP
/// capture sink so that both GUI and MCP see all Maxima I/O.
fn build_output_sink(state: &AppState) -> Arc<dyn OutputSink> {
    let tauri_sink: Arc<dyn OutputSink> =
        Arc::new(TauriOutputSink::new(state.app_handle.clone()));
    let capture_sink: Arc<dyn OutputSink> = state.capture_sink.clone();
    Arc::new(MultiOutputSink::new(vec![tauri_sink, capture_sink]))
}

#[tauri::command]
pub async fn start_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SessionStatus, AppError> {
    let maxima_path = read_maxima_path(&app);
    let backend = read_backend(&app);
    let output_sink = build_output_sink(&state);

    emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session starting...", "session");
    state.session.begin_start().await;

    match MaximaProcess::spawn(backend, maxima_path, output_sink).await {
        Ok(process) => {
            state.session.set_ready(process).await;
            emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session ready", "session");
            Ok(SessionStatus::Ready)
        }
        Err(e) => {
            let msg = e.to_string();
            state.session.set_error(msg.clone()).await;
            emit_app_log(&state.app_handle, &state.app_log, "error", &format!("Maxima session failed: {msg}"), "session");
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn stop_session(state: State<'_, AppState>) -> Result<SessionStatus, AppError> {
    state.session.stop().await?;
    emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session stopped", "session");
    Ok(SessionStatus::Stopped)
}

#[tauri::command]
pub async fn restart_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SessionStatus, AppError> {
    let maxima_path = read_maxima_path(&app);
    let backend = read_backend(&app);
    let output_sink = build_output_sink(&state);

    emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session restarting...", "session");
    state.session.begin_start().await;

    match MaximaProcess::spawn(backend, maxima_path, output_sink).await {
        Ok(process) => {
            state.session.set_ready(process).await;
            emit_app_log(&state.app_handle, &state.app_log, "info", "Maxima session ready", "session");
            Ok(SessionStatus::Ready)
        }
        Err(e) => {
            let msg = e.to_string();
            state.session.set_error(msg.clone()).await;
            emit_app_log(&state.app_handle, &state.app_log, "error", &format!("Maxima session restart failed: {msg}"), "session");
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_session_status(
    state: State<'_, AppState>,
) -> Result<SessionStatus, AppError> {
    Ok(state.session.status())
}
