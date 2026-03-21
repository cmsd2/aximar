use tauri::State;

use crate::error::AppError;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::SessionStatus;
use crate::state::AppState;

#[tauri::command]
pub async fn start_session(state: State<'_, AppState>) -> Result<SessionStatus, AppError> {
    let mut status = state.status.lock().await;
    *status = SessionStatus::Starting;

    match MaximaProcess::spawn().await {
        Ok(process) => {
            let mut guard = state.process.lock().await;
            *guard = Some(process);
            *status = SessionStatus::Ready;
            Ok(SessionStatus::Ready)
        }
        Err(e) => {
            let err_msg = e.to_string();
            *status = SessionStatus::Error(err_msg.clone());
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn stop_session(state: State<'_, AppState>) -> Result<SessionStatus, AppError> {
    let mut guard = state.process.lock().await;
    if let Some(ref mut process) = *guard {
        process.kill().await?;
    }
    *guard = None;

    let mut status = state.status.lock().await;
    *status = SessionStatus::Stopped;
    Ok(SessionStatus::Stopped)
}

#[tauri::command]
pub async fn restart_session(state: State<'_, AppState>) -> Result<SessionStatus, AppError> {
    // Kill existing process
    {
        let mut guard = state.process.lock().await;
        if let Some(ref mut process) = *guard {
            let _ = process.kill().await;
        }
        *guard = None;
    }

    // Start new process
    let mut status = state.status.lock().await;
    *status = SessionStatus::Starting;

    match MaximaProcess::spawn().await {
        Ok(process) => {
            let mut guard = state.process.lock().await;
            *guard = Some(process);
            *status = SessionStatus::Ready;
            Ok(SessionStatus::Ready)
        }
        Err(e) => {
            let err_msg = e.to_string();
            *status = SessionStatus::Error(err_msg.clone());
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_session_status(state: State<'_, AppState>) -> Result<SessionStatus, AppError> {
    let status = state.status.lock().await;
    Ok(status.clone())
}
