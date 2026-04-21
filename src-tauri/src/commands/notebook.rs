use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use aximar_core::error::AppError;
use aximar_core::evaluation::evaluate_cell;
use aximar_core::maxima::types::EvalResult;
use aximar_core::safety;

use aximar_core::commands::{CommandEffect, NotebookCommand};
use aximar_core::notebook::CellType;
use aximar_core::notebook::Notebook;
use aximar_core::registry::{NotebookContextRef, NotebookInfo};

use crate::mcp::sync::{notebook_state_payload, SyncCell};
use crate::state::AppState;

use super::config::{read_backend, read_eval_timeout, read_maxima_path};
use super::session::ensure_session;

// ── Event types ──────────────────────────────────────────────────────

/// Payload emitted after every notebook command application.
#[derive(Debug, Clone, Serialize)]
pub struct NotebookStateEvent {
    pub notebook_id: String,
    pub effect: String,
    pub cell_id: Option<String>,
    pub cells: Vec<SyncCell>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub trusted: bool,
}

/// Emit a `notebook-state-changed` event with the current notebook state.
pub fn emit_notebook_state(
    app_handle: &AppHandle,
    notebook_id: &str,
    nb: &Notebook,
    effect: &CommandEffect,
) {
    let payload = notebook_state_payload(nb);
    let event = NotebookStateEvent {
        notebook_id: notebook_id.to_string(),
        effect: effect.effect_name().to_string(),
        cell_id: effect.cell_id().map(|s| s.to_string()),
        cells: payload.cells,
        can_undo: nb.can_undo(),
        can_redo: nb.can_redo(),
        trusted: nb.trusted(),
    };
    let _ = app_handle.emit("notebook-state-changed", event);
}

// ── Helper ───────────────────────────────────────────────────────────

/// Resolve notebook context: if notebook_id is provided, use it; otherwise use the active notebook.
async fn resolve_context(
    state: &AppState,
    notebook_id: Option<String>,
) -> Result<NotebookContextRef, AppError> {
    let reg = state.registry.lock().await;
    reg.resolve(notebook_id.as_deref())
        .map_err(|e| AppError::CommunicationError(e))
}

// ── Tauri commands ───────────────────────────────────────────────────

/// Return the current notebook state without modifying anything.
/// Used by the frontend on mount to sync initial state.
#[tauri::command]
pub async fn nb_get_state(
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<NotebookStateEvent, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let nb = ctx.notebook.lock().await;
    let payload = notebook_state_payload(&nb);
    Ok(NotebookStateEvent {
        notebook_id: ctx.id.clone(),
        effect: "notebook_replaced".to_string(),
        cell_id: None,
        cells: payload.cells,
        can_undo: nb.can_undo(),
        can_redo: nb.can_redo(),
        trusted: nb.trusted(),
    })
}

#[derive(Debug, Serialize)]
pub struct NbAddCellResult {
    pub cell_id: String,
}

#[tauri::command]
pub async fn nb_add_cell(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    cell_type: Option<String>,
    input: Option<String>,
    after_cell_id: Option<String>,
    before_cell_id: Option<String>,
) -> Result<NbAddCellResult, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let ct = match cell_type.as_deref() {
        Some("markdown") => CellType::Markdown,
        _ => CellType::Code,
    };
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::AddCell {
            cell_type: ct,
            input: input.unwrap_or_default(),
            after_cell_id,
            before_cell_id,
        })
        .map_err(|e| AppError::CommunicationError(e))?;
    let cell_id = match &effect {
        CommandEffect::CellAdded { cell_id } => cell_id.clone(),
        _ => String::new(),
    };
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    Ok(NbAddCellResult { cell_id })
}

#[tauri::command]
pub async fn nb_delete_cell(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    cell_id: String,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::DeleteCell { cell_id })
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    Ok(())
}

#[tauri::command]
pub async fn nb_move_cell(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    cell_id: String,
    direction: String,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::MoveCell {
            cell_id,
            direction,
        })
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    Ok(())
}

#[tauri::command]
pub async fn nb_toggle_cell_type(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    cell_id: String,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::ToggleCellType { cell_id })
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    Ok(())
}

#[tauri::command]
pub async fn nb_update_cell_input(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    cell_id: String,
    input: String,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::UpdateCellInput { cell_id, input })
        .map_err(|e| AppError::CommunicationError(e))?;
    // Only emit if something actually changed
    if !matches!(effect, CommandEffect::NoOp { .. }) {
        emit_notebook_state(&app, &ctx.id, &nb, &effect);
    }
    Ok(())
}

#[tauri::command]
pub async fn nb_undo(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::Undo)
        .map_err(|e| AppError::CommunicationError(e))?;
    if !matches!(effect, CommandEffect::NoOp { .. }) {
        emit_notebook_state(&app, &ctx.id, &nb, &effect);
    }
    Ok(())
}

#[tauri::command]
pub async fn nb_redo(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::Redo)
        .map_err(|e| AppError::CommunicationError(e))?;
    if !matches!(effect, CommandEffect::NoOp { .. }) {
        emit_notebook_state(&app, &ctx.id, &nb, &effect);
    }
    Ok(())
}

#[tauri::command]
pub async fn nb_new_notebook(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::NewNotebook)
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct LoadCellOutput {
    pub text_output: String,
    pub latex: Option<String>,
    pub plot_data: Option<String>,
    pub plot_svg: Option<String>,
    pub image_png: Option<String>,
    pub execution_count: Option<u32>,
}

#[derive(Debug, serde::Deserialize)]
pub struct LoadCell {
    pub id: String,
    pub cell_type: String,
    pub input: String,
    #[serde(default)]
    pub output: Option<LoadCellOutput>,
}

#[tauri::command]
pub async fn nb_load_cells(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    cells: Vec<LoadCell>,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let cell_tuples: Vec<(String, CellType, String, Option<aximar_core::notebook::CellOutput>)> = cells
        .into_iter()
        .map(|c| {
            let ct = match c.cell_type.as_str() {
                "markdown" => CellType::Markdown,
                _ => CellType::Code,
            };
            let output = c.output.map(|o| aximar_core::notebook::CellOutput {
                text_output: o.text_output,
                latex: o.latex,
                execution_count: o.execution_count,
                plot_svg: o.plot_svg,
                plot_data: o.plot_data,
                image_png: o.image_png,
                error: None,
                is_error: false,
                duration_ms: 0,
                output_label: None,
            });
            (c.id, ct, c.input, output)
        })
        .collect();
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::LoadCells {
            cells: cell_tuples,
        })
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    Ok(())
}

/// Result of running a cell — either evaluated or needs notebook trust.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunCellResult {
    Evaluated(EvalResult),
    NeedsNotebookTrust { dangerous_functions: Vec<String> },
}

/// Run a cell: checks for dangerous functions, sets status to Running, evaluates, sets output.
/// If the notebook is untrusted and the cell contains dangerous functions, returns NeedsNotebookTrust.
#[tauri::command]
pub async fn nb_run_cell(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    cell_id: String,
) -> Result<RunCellResult, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;

    // Read cell input + notebook trust flag
    let (input, notebook_trusted) = {
        let nb = ctx.notebook.lock().await;
        let cell = nb.get_cell(&cell_id)
            .ok_or_else(|| AppError::CellNotFound(cell_id.clone()))?;
        (cell.input.clone(), nb.trusted())
    };

    // Safety check: skip if notebook is trusted
    if !notebook_trusted {
        let dangerous = safety::detect_dangerous_calls(&input, Some(&state.packages));
        if !dangerous.is_empty() {
            let func_names: Vec<String> = dangerous.iter().map(|d| d.function_name.clone()).collect();
            return Ok(RunCellResult::NeedsNotebookTrust { dangerous_functions: func_names });
        }
    }

    let eval_timeout = read_eval_timeout(&app);
    let backend = read_backend(&app);
    let maxima_path = read_maxima_path(&app);
    ensure_session(&state, &ctx, backend, maxima_path, eval_timeout).await?;

    let result = evaluate_cell(&ctx, &cell_id, &state.catalog, &state.packages, eval_timeout).await?;

    // Emit transport-specific notifications for all effects
    let nb = ctx.notebook.lock().await;
    for effect in &result.effects {
        emit_notebook_state(&app, &ctx.id, &nb, effect);
    }

    Ok(RunCellResult::Evaluated(result.eval_result))
}

/// Set the notebook-level trust flag.
#[tauri::command]
pub async fn nb_trust_notebook(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    trusted: bool,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb.apply(NotebookCommand::TrustNotebook { trusted })
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    Ok(())
}

// ── Notebook lifecycle commands ──────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct NbCreateResult {
    pub notebook_id: String,
}

#[tauri::command]
pub async fn nb_create(
    state: State<'_, AppState>,
) -> Result<NbCreateResult, AppError> {
    let mut reg = state.registry.lock().await;
    let id = reg.create();
    Ok(NbCreateResult { notebook_id: id })
}

#[tauri::command]
pub async fn nb_close(
    state: State<'_, AppState>,
    notebook_id: String,
) -> Result<(), AppError> {
    let ctx = {
        let mut reg = state.registry.lock().await;
        reg.close(&notebook_id)
            .map_err(|e| AppError::CommunicationError(e))?
    };
    // Stop the Maxima session for the closed notebook
    let _ = ctx.session.stop().await;
    Ok(())
}

#[tauri::command]
pub async fn nb_list(
    state: State<'_, AppState>,
) -> Result<Vec<NotebookInfo>, AppError> {
    let reg = state.registry.lock().await;
    Ok(reg.list())
}

#[tauri::command]
pub async fn nb_set_active(
    state: State<'_, AppState>,
    notebook_id: String,
) -> Result<(), AppError> {
    let mut reg = state.registry.lock().await;
    reg.set_active(&notebook_id)
        .map_err(|e| AppError::CommunicationError(e))
}
