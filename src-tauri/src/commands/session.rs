use tauri::State;

use crate::commands::config::{read_backend, read_maxima_path};
use crate::error::AppError;
use crate::maxima::backend::Backend;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::SessionStatus;
use crate::state::AppState;

#[tauri::command]
pub async fn start_session(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<SessionStatus, AppError> {
    let maxima_path = read_maxima_path(&app);
    let (backend_str, docker_image, wsl_distro, container_engine) = read_backend(&app);
    let backend = Backend::from_config(&backend_str, &docker_image, &wsl_distro, &container_engine);

    let mut status = state.status.lock().await;
    *status = SessionStatus::Starting;

    match MaximaProcess::spawn(backend, maxima_path, Some(app)).await {
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
pub async fn restart_session(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<SessionStatus, AppError> {
    let maxima_path = read_maxima_path(&app);
    let (backend_str, docker_image, wsl_distro, container_engine) = read_backend(&app);
    let backend = Backend::from_config(&backend_str, &docker_image, &wsl_distro, &container_engine);

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

    match MaximaProcess::spawn(backend, maxima_path, Some(app.clone())).await {
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
