use tauri::State;

use crate::error::AppError;
use crate::maxima::protocol;
use crate::maxima::types::EvalResult;
use crate::state::AppState;

#[tauri::command]
pub async fn evaluate_expression(
    state: State<'_, AppState>,
    cell_id: String,
    expression: String,
) -> Result<EvalResult, AppError> {
    let mut guard = state.process.lock().await;
    let process = guard
        .as_mut()
        .ok_or(AppError::ProcessNotRunning)?;

    protocol::evaluate(process, &cell_id, &expression).await
}
