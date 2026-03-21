use tauri::State;

use crate::commands::config::read_eval_timeout;
use crate::error::AppError;
use crate::maxima::protocol;
use crate::maxima::types::EvalResult;
use crate::state::AppState;

#[tauri::command]
pub async fn evaluate_expression(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    cell_id: String,
    expression: String,
) -> Result<EvalResult, AppError> {
    let eval_timeout = read_eval_timeout(&app);
    let mut guard = state.process.lock().await;
    let process = guard
        .as_mut()
        .ok_or(AppError::ProcessNotRunning)?;

    protocol::evaluate(process, &cell_id, &expression, &state.catalog, eval_timeout).await
}
