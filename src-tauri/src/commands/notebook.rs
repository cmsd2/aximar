use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use aximar_core::error::AppError;
use aximar_core::evaluation::{
    check_cell_trust, evaluate_cell, run_all, RunAllOptions, RunAllReport,
};
use aximar_core::maxima::types::EvalResult;
use aximar_core::safety::{TrustDecision, TrustMode};

use aximar_core::commands::{CommandEffect, EffectSink, NotebookCommand};
use aximar_core::notebook::CellType;
use aximar_core::notebook::Notebook;
use aximar_core::notebooks::{data as notebook_data, io as notebook_io};
use aximar_core::registry::{NotebookContextRef, NotebookInfo};

use crate::mcp::sync::{notebook_state_payload, SyncCell};
use crate::state::AppState;

use super::config::read_session_settings;
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
    /// With `trust_required`: the dangerous functions that blocked the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_functions: Option<Vec<String>>,
    /// With `path_changed`: the notebook's file path, absent when cleared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Emit a `notebook-state-changed` event with the current notebook state.
pub fn emit_notebook_state(
    app_handle: &AppHandle,
    notebook_id: &str,
    nb: &Notebook,
    effect: &CommandEffect,
) {
    let _ = app_handle.emit(
        "notebook-state-changed",
        notebook_state_event(notebook_id, nb, effect),
    );
}

/// The event payload for an effect on a notebook.
fn notebook_state_event(
    notebook_id: &str,
    nb: &Notebook,
    effect: &CommandEffect,
) -> NotebookStateEvent {
    let payload = notebook_state_payload(nb);
    NotebookStateEvent {
        notebook_id: notebook_id.to_string(),
        effect: effect.effect_name().to_string(),
        cell_id: effect.cell_id().map(|s| s.to_string()),
        cells: payload.cells,
        can_undo: nb.can_undo(),
        can_redo: nb.can_redo(),
        trusted: nb.trusted(),
        trust_functions: match effect {
            CommandEffect::TrustRequired { functions, .. } => Some(functions.clone()),
            _ => None,
        },
        path: match effect {
            CommandEffect::PathChanged { path } => path.clone(),
            _ => None,
        },
    }
}

/// Emits each effect to the window as the operation produces it.
struct WindowEffects<'a>(&'a AppHandle);

impl EffectSink for WindowEffects<'_> {
    fn effect(&self, notebook_id: &str, notebook: &Notebook, effect: &CommandEffect) {
        emit_notebook_state(self.0, notebook_id, notebook, effect);
    }
}

// ── Helper ───────────────────────────────────────────────────────────

/// Resolve notebook context: if notebook_id is provided, use it; otherwise use the active notebook.
async fn resolve_context(
    state: &AppState,
    notebook_id: Option<String>,
) -> Result<NotebookContextRef, AppError> {
    let reg = state.registry.lock().await;
    reg.resolve(notebook_id.as_deref())
        .map_err(AppError::CommunicationError)
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
        trust_functions: None,
        path: None,
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
        .map_err(AppError::CommunicationError)?;
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
        .map_err(AppError::CommunicationError)?;
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
        .apply(NotebookCommand::MoveCell { cell_id, direction })
        .map_err(AppError::CommunicationError)?;
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
        .map_err(AppError::CommunicationError)?;
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
        .map_err(AppError::CommunicationError)?;
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
        .map_err(AppError::CommunicationError)?;
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
        .map_err(AppError::CommunicationError)?;
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
        .map_err(AppError::CommunicationError)?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    drop(nb);
    // A new notebook belongs to no file
    set_path(&app, &state, &ctx.id, None).await
}

/// Record the file a notebook was opened from or saved to, or clear it
/// (`None`).  The window calls this after its own file operations; the
/// `path_changed` event then sets the tab's title and save path, as it does
/// when MCP opens or saves.  Maxima's working directory follows from the
/// session's next start.
#[tauri::command]
pub async fn nb_set_path(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    path: Option<String>,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    set_path(&app, &state, &ctx.id, path).await
}

async fn set_path(
    app: &AppHandle,
    state: &AppState,
    notebook_id: &str,
    path: Option<String>,
) -> Result<(), AppError> {
    let effect = state
        .registry
        .lock()
        .await
        .set_path(notebook_id, path.map(std::path::PathBuf::from))
        .map_err(AppError::CommunicationError)?;
    let ctx = resolve_context(state, Some(notebook_id.to_string())).await?;
    let nb = ctx.notebook.lock().await;
    emit_notebook_state(app, notebook_id, &nb, &effect);
    Ok(())
}

/// Open a notebook file into a notebook: read it, load its cells and record
/// its path, in one command.  The `path_changed` event then titles the tab.
/// When some cells had no usable ID and got new ones, an `ids_replaced`
/// event follows: the notebook no longer matches its file, so the window
/// marks it changed and the next save writes good IDs.
#[tauri::command]
pub async fn nb_open(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    path: String,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let file = notebook_io::open(Path::new(&path), &state.catalog, &state.packages)
        .map_err(AppError::NotebookFile)?;
    let ids_replaced = file.ids_replaced;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::LoadNotebook { file: file.into() })
        .map_err(AppError::CommunicationError)?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    drop(nb);
    set_path(&app, &state, &ctx.id, Some(path)).await?;
    if ids_replaced {
        let nb = ctx.notebook.lock().await;
        let mut event = notebook_state_event(&ctx.id, &nb, &effect);
        event.effect = "ids_replaced".to_string();
        let _ = app.emit("notebook-state-changed", event);
    }
    Ok(())
}

/// Save a notebook to a file and record the path.  The window flushes its
/// pending input edits first, so the backend's notebook is the one saved.
#[tauri::command]
pub async fn nb_save(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    path: String,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let nb = ctx.notebook.lock().await;
    notebook_io::save(Path::new(&path), &nb).map_err(AppError::NotebookFile)?;
    drop(nb);
    set_path(&app, &state, &ctx.id, Some(path)).await
}

/// Load a built-in template into a notebook.  The notebook then belongs to
/// no file.
#[tauri::command]
pub async fn nb_load_template(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    template_id: String,
) -> Result<(), AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let file = notebook_data::template_file(&template_id)
        .ok_or_else(|| AppError::NotebookFile(format!("No template named '{template_id}'")))?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb
        .apply(NotebookCommand::LoadNotebook { file: file.into() })
        .map_err(AppError::CommunicationError)?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    drop(nb);
    set_path(&app, &state, &ctx.id, None).await
}

/// Result of running a cell — either evaluated or needs notebook trust.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunCellResult {
    Evaluated(Box<EvalResult>),
    NeedsNotebookTrust { dangerous_functions: Vec<String> },
}

/// Run a cell: checks for dangerous functions, sets status to Running, evaluates, sets output.
/// If the notebook is untrusted and the cell contains dangerous functions, returns
/// NeedsNotebookTrust; the window's banner shows from the `trust_required` event.
#[tauri::command]
pub async fn nb_run_cell(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
    cell_id: String,
) -> Result<RunCellResult, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;

    let decision = check_cell_trust(
        &ctx,
        &cell_id,
        &state.packages,
        TrustMode::Interactive,
        &WindowEffects(&app),
    )
    .await?;
    if let TrustDecision::NeedsTrust { functions } = decision {
        return Ok(RunCellResult::NeedsNotebookTrust {
            dangerous_functions: functions,
        });
    }

    let settings = read_session_settings(&app);
    let eval_timeout = settings.eval_timeout;
    ensure_session(&state, &ctx, &settings).await?;

    // evaluate_cell reports Running, the output and any error to the window
    // as each happens
    let result = evaluate_cell(
        &ctx,
        &cell_id,
        &state.catalog,
        &state.packages,
        eval_timeout,
        &WindowEffects(&app),
    )
    .await?;

    Ok(RunCellResult::Evaluated(Box::new(result.eval_result)))
}

/// Run every code cell in order, stopping at the first error, trust block
/// or failure, as MCP's `run_all_cells` does.  Each cell's status reaches
/// the window as it changes; `cancel_evaluation` stops the run after the
/// cell running now.  The caller flushes pending edits first.
#[tauri::command]
pub async fn nb_run_all(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: Option<String>,
) -> Result<RunAllReport, AppError> {
    let ctx = resolve_context(&state, notebook_id).await?;
    let settings = read_session_settings(&app);
    ensure_session(&state, &ctx, &settings).await?;
    let opts = RunAllOptions {
        catalog: &state.catalog,
        packages: &state.packages,
        eval_timeout: settings.eval_timeout,
        mode: TrustMode::Interactive,
        effects: &WindowEffects(&app),
    };
    Ok(run_all(&ctx, &opts, &mut |_| {}).await)
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
    let effect = nb
        .apply(NotebookCommand::TrustNotebook { trusted })
        .map_err(AppError::CommunicationError)?;
    emit_notebook_state(&app, &ctx.id, &nb, &effect);
    Ok(())
}

// ── Notebook lifecycle commands ──────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct NbCreateResult {
    pub notebook_id: String,
}

#[tauri::command]
pub async fn nb_create(state: State<'_, AppState>) -> Result<NbCreateResult, AppError> {
    let mut reg = state.registry.lock().await;
    let id = reg.create();
    Ok(NbCreateResult { notebook_id: id })
}

#[tauri::command]
pub async fn nb_close(state: State<'_, AppState>, notebook_id: String) -> Result<(), AppError> {
    let ctx = {
        let mut reg = state.registry.lock().await;
        reg.close(&notebook_id)
            .map_err(AppError::CommunicationError)?
    };
    // Stop the Maxima session for the closed notebook
    let _ = ctx.session.stop().await;
    Ok(())
}

#[tauri::command]
pub async fn nb_list(state: State<'_, AppState>) -> Result<Vec<NotebookInfo>, AppError> {
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
        .map_err(AppError::CommunicationError)
}
