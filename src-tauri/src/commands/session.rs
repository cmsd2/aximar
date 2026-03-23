use tauri::State;

use crate::commands::config::{read_backend, read_maxima_path};
use crate::error::AppError;
use crate::maxima::backend::Backend;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::SessionStatus;
use crate::state::AppState;

#[tauri::command]
pub async fn start_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SessionStatus, AppError> {
    let maxima_path = read_maxima_path(&app);
    let (backend_str, docker_image, wsl_distro, container_engine) = read_backend(&app);
    let backend = Backend::from_config(&backend_str, &docker_image, &wsl_distro, &container_engine);

    state.session.begin_start().await;

    match MaximaProcess::spawn(backend, maxima_path, Some(app)).await {
        Ok(process) => {
            state.session.set_ready(process).await;
            Ok(SessionStatus::Ready)
        }
        Err(e) => {
            state.session.set_error(e.to_string()).await;
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn stop_session(state: State<'_, AppState>) -> Result<SessionStatus, AppError> {
    state.session.stop().await?;
    Ok(SessionStatus::Stopped)
}

#[tauri::command]
pub async fn restart_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SessionStatus, AppError> {
    let maxima_path = read_maxima_path(&app);
    let (backend_str, docker_image, wsl_distro, container_engine) = read_backend(&app);
    let backend = Backend::from_config(&backend_str, &docker_image, &wsl_distro, &container_engine);

    state.session.begin_start().await;

    match MaximaProcess::spawn(backend, maxima_path, Some(app.clone())).await {
        Ok(process) => {
            state.session.set_ready(process).await;
            Ok(SessionStatus::Ready)
        }
        Err(e) => {
            state.session.set_error(e.to_string()).await;
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
