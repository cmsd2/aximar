use tauri::State;

use aximar_core::error::AppError;
use aximar_core::maxima::labels::{self, LabelContext};
use aximar_core::maxima::protocol;
use aximar_core::maxima::types::EvalResult;
use aximar_core::registry::NotebookContextRef;
use crate::commands::config::{read_backend, read_eval_timeout, read_maxima_path};
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
    label_context: Option<LabelContext>,
) -> Result<EvalResult, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let eval_timeout = read_eval_timeout(&app);
    let backend = read_backend(&app);
    let maxima_path = read_maxima_path(&app);
    ensure_session(&state, &ctx, backend, maxima_path, eval_timeout).await?;

    let expression = match label_context {
        Some(ref ctx) => labels::rewrite_labels(&expression, ctx),
        None => expression,
    };

    let mut guard = ctx.session.lock().await;
    let process = guard.try_begin_eval()?;
    let result = protocol::evaluate(process, &cell_id, &expression, &state.catalog, eval_timeout).await;
    guard.end_eval();
    result
}

/// Fire a cooperative cancel at the running Maxima process.  Writes
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
    ctx.session.request_cancel()
}
