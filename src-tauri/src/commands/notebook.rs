use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use aximar_core::error::AppError;
use aximar_core::maxima::labels::{rewrite_labels, LabelContext};
use aximar_core::maxima::protocol;
use aximar_core::maxima::types::EvalResult;
use aximar_core::maxima::unicode::unicode_to_maxima;

use aximar_core::commands::{CommandEffect, NotebookCommand};
use aximar_core::notebook::{CellOutput, CellStatus, CellType, Notebook};

use crate::mcp::sync::{notebook_state_payload, SyncCell};
use crate::state::AppState;

use super::config::read_eval_timeout;

// ── Event types ──────────────────────────────────────────────────────

/// Payload emitted after every notebook command application.
#[derive(Debug, Clone, Serialize)]
pub struct NotebookStateEvent {
    pub effect: String,
    pub cell_id: Option<String>,
    pub cells: Vec<SyncCell>,
    pub can_undo: bool,
    pub can_redo: bool,
}

/// Emit a `notebook-state-changed` event with the current notebook state.
pub fn emit_notebook_state(
    app_handle: &AppHandle,
    nb: &Notebook,
    effect: &CommandEffect,
) {
    let payload = notebook_state_payload(nb);
    let event = NotebookStateEvent {
        effect: effect.effect_name().to_string(),
        cell_id: effect.cell_id().map(|s| s.to_string()),
        cells: payload.cells,
        can_undo: nb.can_undo(),
        can_redo: nb.can_redo(),
    };
    let _ = app_handle.emit("notebook-state-changed", event);
}

// ── Tauri commands ───────────────────────────────────────────────────

/// Return the current notebook state without modifying anything.
/// Used by the frontend on mount to sync initial state.
#[tauri::command]
pub async fn nb_get_state(
    state: State<'_, AppState>,
) -> Result<NotebookStateEvent, AppError> {
    let nb = state.notebook.lock().await;
    let payload = notebook_state_payload(&nb);
    Ok(NotebookStateEvent {
        effect: "notebook_replaced".to_string(),
        cell_id: None,
        cells: payload.cells,
        can_undo: nb.can_undo(),
        can_redo: nb.can_redo(),
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
    cell_type: Option<String>,
    input: Option<String>,
    after_cell_id: Option<String>,
) -> Result<NbAddCellResult, AppError> {
    let ct = match cell_type.as_deref() {
        Some("markdown") => CellType::Markdown,
        _ => CellType::Code,
    };
    let mut nb = state.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::AddCell {
            cell_type: ct,
            input: input.unwrap_or_default(),
            after_cell_id,
        })
        .map_err(|e| AppError::CommunicationError(e))?;
    let cell_id = match &effect {
        CommandEffect::CellAdded { cell_id } => cell_id.clone(),
        _ => String::new(),
    };
    emit_notebook_state(&app, &nb, &effect);
    Ok(NbAddCellResult { cell_id })
}

#[tauri::command]
pub async fn nb_delete_cell(
    app: AppHandle,
    state: State<'_, AppState>,
    cell_id: String,
) -> Result<(), AppError> {
    let mut nb = state.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::DeleteCell { cell_id })
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &nb, &effect);
    Ok(())
}

#[tauri::command]
pub async fn nb_move_cell(
    app: AppHandle,
    state: State<'_, AppState>,
    cell_id: String,
    direction: String,
) -> Result<(), AppError> {
    let mut nb = state.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::MoveCell {
            cell_id,
            direction,
        })
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &nb, &effect);
    Ok(())
}

#[tauri::command]
pub async fn nb_toggle_cell_type(
    app: AppHandle,
    state: State<'_, AppState>,
    cell_id: String,
) -> Result<(), AppError> {
    let mut nb = state.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::ToggleCellType { cell_id })
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &nb, &effect);
    Ok(())
}

#[tauri::command]
pub async fn nb_update_cell_input(
    app: AppHandle,
    state: State<'_, AppState>,
    cell_id: String,
    input: String,
) -> Result<(), AppError> {
    let mut nb = state.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::UpdateCellInput { cell_id, input })
        .map_err(|e| AppError::CommunicationError(e))?;
    // Only emit if something actually changed
    if !matches!(effect, CommandEffect::NoOp { .. }) {
        emit_notebook_state(&app, &nb, &effect);
    }
    Ok(())
}

#[tauri::command]
pub async fn nb_undo(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut nb = state.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::Undo)
        .map_err(|e| AppError::CommunicationError(e))?;
    if !matches!(effect, CommandEffect::NoOp { .. }) {
        emit_notebook_state(&app, &nb, &effect);
    }
    Ok(())
}

#[tauri::command]
pub async fn nb_redo(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut nb = state.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::Redo)
        .map_err(|e| AppError::CommunicationError(e))?;
    if !matches!(effect, CommandEffect::NoOp { .. }) {
        emit_notebook_state(&app, &nb, &effect);
    }
    Ok(())
}

#[tauri::command]
pub async fn nb_new_notebook(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut nb = state.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::NewNotebook)
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &nb, &effect);
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct LoadCell {
    pub id: String,
    pub cell_type: String,
    pub input: String,
}

#[tauri::command]
pub async fn nb_load_cells(
    app: AppHandle,
    state: State<'_, AppState>,
    cells: Vec<LoadCell>,
) -> Result<(), AppError> {
    let cell_tuples: Vec<(String, CellType, String)> = cells
        .into_iter()
        .map(|c| {
            let ct = match c.cell_type.as_str() {
                "markdown" => CellType::Markdown,
                _ => CellType::Code,
            };
            (c.id, ct, c.input)
        })
        .collect();
    let mut nb = state.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::LoadCells {
            cells: cell_tuples,
        })
        .map_err(|e| AppError::CommunicationError(e))?;
    emit_notebook_state(&app, &nb, &effect);
    Ok(())
}

/// Run a cell: sets status to Running, evaluates, sets output.
/// Returns the EvalResult for frontend logging purposes.
#[tauri::command]
pub async fn nb_run_cell(
    app: AppHandle,
    state: State<'_, AppState>,
    cell_id: String,
) -> Result<EvalResult, AppError> {
    // 1. Get cell input and validate
    let (input, cell_type) = {
        let nb = state.notebook.lock().await;
        let cell = nb
            .get_cell(&cell_id)
            .ok_or_else(|| AppError::CommunicationError(format!("Cell '{}' not found", cell_id)))?;
        (cell.input.clone(), cell.cell_type)
    };

    if cell_type == CellType::Markdown {
        return Err(AppError::CommunicationError(
            "Cannot run a markdown cell".into(),
        ));
    }

    if input.trim().is_empty() {
        return Err(AppError::CommunicationError("Cell input is empty".into()));
    }

    // 2. Set status to Running
    {
        let mut nb = state.notebook.lock().await;
        let effect = nb
            .apply(NotebookCommand::SetCellStatus {
                cell_id: cell_id.clone(),
                status: CellStatus::Running,
            })
            .map_err(|e| AppError::CommunicationError(e))?;
        emit_notebook_state(&app, &nb, &effect);
    }

    // 3. Prepare label context and rewrite input
    let (exec_count, label_ctx) = {
        let mut nb = state.notebook.lock().await;
        let ec = nb.next_execution_count();
        let ctx = LabelContext {
            label_map: nb.label_map().clone(),
            previous_output_label: nb.previous_output_label(&cell_id),
        };
        (ec, ctx)
    };

    let translated = unicode_to_maxima(&input);
    let rewritten = rewrite_labels(&translated, &label_ctx);
    let eval_timeout = read_eval_timeout(&app);

    // 4. Clear capture and evaluate
    state.capture_sink.take_cell_output();

    let mut guard = state.session.lock().await;
    let process = match guard.try_begin_eval() {
        Ok(p) => p,
        Err(e) => {
            let mut nb = state.notebook.lock().await;
            let effect = nb
                .apply(NotebookCommand::SetCellStatus {
                    cell_id: cell_id.clone(),
                    status: CellStatus::Error,
                })
                .map_err(|e| AppError::CommunicationError(e))?;
            emit_notebook_state(&app, &nb, &effect);
            return Err(e);
        }
    };

    let result = protocol::evaluate(process, &cell_id, &rewritten, &state.catalog, eval_timeout).await;
    guard.end_eval();
    drop(guard);

    // 5. Capture raw output
    let raw_output = state.capture_sink.take_cell_output();

    // 6. Apply output to notebook
    match result {
        Ok(eval_result) => {
            // Record label mapping
            if let Some(ref label) = eval_result.output_label {
                let mut nb = state.notebook.lock().await;
                nb.record_label(exec_count, label.clone());
            }

            let cell_output = CellOutput::from_eval_result(&eval_result, exec_count);
            let mut nb = state.notebook.lock().await;
            let effect = nb
                .apply(NotebookCommand::SetCellOutput {
                    cell_id: cell_id.clone(),
                    output: cell_output,
                    raw_output,
                })
                .map_err(|e| AppError::CommunicationError(e))?;
            emit_notebook_state(&app, &nb, &effect);

            Ok(eval_result)
        }
        Err(e) => {
            let mut nb = state.notebook.lock().await;
            let effect = nb
                .apply(NotebookCommand::SetCellStatus {
                    cell_id: cell_id.clone(),
                    status: CellStatus::Error,
                })
                .map_err(|err| AppError::CommunicationError(err))?;
            emit_notebook_state(&app, &nb, &effect);
            Err(e)
        }
    }
}
