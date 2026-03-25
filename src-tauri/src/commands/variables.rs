use tauri::State;

use aximar_core::error::AppError;
use aximar_core::maxima::protocol;
use aximar_core::registry::NotebookContextRef;
use crate::state::AppState;

/// Resolve notebook context: if notebook_id is provided, use it; otherwise use the active notebook.
async fn resolve_context(
    state: &AppState,
    notebook_id: Option<String>,
) -> Result<NotebookContextRef, AppError> {
    let reg = state.registry.lock().await;
    reg.resolve(notebook_id.as_deref())
        .map_err(|e| AppError::CommunicationError(e))
}

#[tauri::command]
pub async fn list_variables(
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<Vec<String>, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut guard = ctx.session.lock().await;
    let process = guard.process_mut()?;

    protocol::query_variables(process).await
}

#[tauri::command]
pub async fn kill_variable(
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    name: String,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut guard = ctx.session.lock().await;
    let process = guard.process_mut()?;

    protocol::kill_variable(process, &name).await
}

#[tauri::command]
pub async fn kill_all_variables(
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut guard = ctx.session.lock().await;
    let process = guard.process_mut()?;

    protocol::kill_all_variables(process).await
}
