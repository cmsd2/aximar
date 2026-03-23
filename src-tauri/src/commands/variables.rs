use tauri::State;

use crate::error::AppError;
use crate::maxima::protocol;
use crate::state::AppState;

#[tauri::command]
pub async fn list_variables(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    let mut guard = state.session.lock().await;
    let process = guard.process_mut()?;

    protocol::query_variables(process).await
}

#[tauri::command]
pub async fn kill_variable(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), AppError> {
    let mut guard = state.session.lock().await;
    let process = guard.process_mut()?;

    protocol::kill_variable(process, &name).await
}

#[tauri::command]
pub async fn kill_all_variables(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut guard = state.session.lock().await;
    let process = guard.process_mut()?;

    protocol::kill_all_variables(process).await
}
