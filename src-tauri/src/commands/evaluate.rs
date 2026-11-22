use tauri::State;

use aximar_core::error::AppError;
use aximar_core::evaluation::prepare_input;
use aximar_core::maxima::labels::UnresolvedLabel;
use aximar_core::maxima::protocol;
use aximar_core::maxima::types::EvalResult;
use aximar_core::registry::NotebookContextRef;
use aximar_core::safety::{self, TrustDecision, TrustMode};
use crate::commands::config::read_session_settings;
use crate::commands::session::ensure_session;
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
pub async fn evaluate_expression(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    cell_id: String,
    expression: String,
) -> Result<EvalResult, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let (decision, labels) = {
        let nb = ctx.notebook.lock().await;
        let decision = safety::check_trust(&nb, &expression, &state.packages, TrustMode::Interactive);
        (decision, nb.expression_label_context())
    };
    if let TrustDecision::NeedsTrust { functions } = decision {
        return Ok(EvalResult {
            cell_id,
            error: Some(format!(
                "Dangerous function(s) blocked: {}. Trust the notebook to run them.",
                functions.join(", ")
            )),
            is_error: true,
            ..Default::default()
        });
    }
    // Unicode and label references, as for cells; a label with no value is
    // reported as the result's error
    let expression = match prepare_input(&expression, &labels) {
        Ok(prepared) => prepared,
        Err(UnresolvedLabel(message)) => {
            return Ok(EvalResult {
                cell_id,
                error: Some(message),
                is_error: true,
                ..Default::default()
            });
        }
    };
    let settings = read_session_settings(&app);
    let eval_timeout = settings.eval_timeout;
    ensure_session(&state, &ctx, &settings).await?;

    let mut guard = ctx.session.lock().await;
    let process = guard.try_begin_eval()?;
    let result = protocol::evaluate(process, &cell_id, &expression, &state.catalog, eval_timeout).await;
    guard.end_eval();
    result
}

/// Stop a running Run All after the current cell, and fire a cooperative
/// cancel at the running Maxima process.  Writes
/// one byte through the fd-4 cancel pipe; the in-kernel watcher sets
/// `*cancel-flag*`, the next `check_cancel()` in user code raises
/// `cancellation-requested`, and `protocol::evaluate` returns
/// `AppError::EvalCancelled` through the Phase-B.1 overlay.
///
/// Best-effort: returns `Ok(())` even when the cancel transport
/// isn't wired (kernel-events disabled, non-Local backend, non-Unix
/// platform) so a stop button in the UI doesn't error on builds
/// without the envelope channel.  Cancellation is cooperative, so
/// nothing happens until library code or the user's loop calls
/// `check_cancel()` — a tight Maxima `for ... do (...)` without
/// `check_cancel()` calls is uncancellable today (the upstream
/// `mdo` patch would change that).
#[tauri::command]
pub async fn cancel_evaluation(
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    // Stop a Run All after this cell, as well as the cell itself
    ctx.cancel_run_all();
    ctx.session.request_cancel()
}
