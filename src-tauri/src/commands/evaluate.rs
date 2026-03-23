use tauri::State;

use crate::commands::config::read_eval_timeout;
use crate::error::AppError;
use crate::maxima::labels::{self, LabelContext};
use crate::maxima::protocol;
use crate::maxima::types::EvalResult;
use crate::state::AppState;

#[tauri::command]
pub async fn evaluate_expression(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    cell_id: String,
    expression: String,
    label_context: Option<LabelContext>,
) -> Result<EvalResult, AppError> {
    let eval_timeout = read_eval_timeout(&app);

    let expression = match label_context {
        Some(ref ctx) => labels::rewrite_labels(&expression, ctx),
        None => expression,
    };

    let mut guard = state.session.lock().await;
    let process = guard.try_begin_eval()?;
    let result = protocol::evaluate(process, &cell_id, &expression, &state.catalog, eval_timeout).await;
    guard.end_eval();
    result
}
