use tauri::State;

use aximar_core::error::AppError;
use aximar_core::maxima::protocol;
use aximar_core::maxima::types::EvalResult;
use aximar_core::registry::NotebookContextRef;
use crate::commands::config::{read_backend, read_eval_timeout, read_maxima_path};
use crate::commands::session::ensure_session;
use crate::state::AppState;

fn is_valid_signal_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_valid_view_id(view_id: &str) -> bool {
    view_id.starts_with("v_")
        && view_id.len() > 2
        && view_id.len() <= 64
        && view_id[2..].chars().all(|c| c.is_ascii_digit())
}

async fn resolve_context(
    state: &AppState,
    notebook_id: Option<String>,
) -> Result<NotebookContextRef, AppError> {
    let reg = state.registry.lock().await;
    reg.resolve(notebook_id.as_deref())
        .map_err(AppError::CommunicationError)
}

/// Update a registered reactive signal and re-render the view that uses it.
///
/// Drives the Maxima session through the standard evaluate path, so the
/// returned EvalResult carries the freshly-generated `.plotly.json` content
/// in `plot_data` exactly the same way an initial `ax_draw2d` call would.
#[tauri::command]
pub async fn set_signal_and_replot(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    view_id: String,
    name: String,
    value: f64,
) -> Result<EvalResult, AppError> {
    if !is_valid_signal_name(&name) {
        return Err(AppError::CommunicationError(format!(
            "Invalid signal name: {name}"
        )));
    }
    if !is_valid_view_id(&view_id) {
        return Err(AppError::CommunicationError(format!(
            "Invalid view id: {view_id}"
        )));
    }
    if !value.is_finite() {
        return Err(AppError::CommunicationError(
            "Signal value must be a finite number".to_string(),
        ));
    }

    let ctx = resolve_context(&state, notebook_id).await?;
    let eval_timeout = read_eval_timeout(&app);
    let backend = read_backend(&app);
    let maxima_path = read_maxima_path(&app);
    ensure_session(&state, &ctx, backend, maxima_path, eval_timeout).await?;

    let expression = format!(
        "signal_set(\"{name}\", {value})$\nax__replot_2d(\"{view_id}\");"
    );

    let mut guard = ctx.session.lock().await;
    let process = guard.try_begin_eval()?;
    let cell_id = format!("__reactive_{view_id}__");
    let result =
        protocol::evaluate(process, &cell_id, &expression, &state.catalog, eval_timeout).await;
    guard.end_eval();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_name_validation() {
        assert!(is_valid_signal_name("freq"));
        assert!(is_valid_signal_name("k1"));
        assert!(is_valid_signal_name("alpha_beta"));
        assert!(!is_valid_signal_name(""));
        assert!(!is_valid_signal_name("has space"));
        assert!(!is_valid_signal_name("\")$ kill(all)$ /*"));
        assert!(!is_valid_signal_name(&"x".repeat(200)));
    }

    #[test]
    fn view_id_validation() {
        assert!(is_valid_view_id("v_1"));
        assert!(is_valid_view_id("v_12345"));
        assert!(!is_valid_view_id("v_"));
        assert!(!is_valid_view_id("v_abc"));
        assert!(!is_valid_view_id("vv_1"));
        assert!(!is_valid_view_id("../../etc/passwd"));
    }
}
