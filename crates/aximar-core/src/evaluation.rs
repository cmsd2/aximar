use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::commands::{CommandEffect, NotebookCommand};
use crate::error::AppError;
use crate::maxima::labels::{rewrite_labels, LabelContext};
use crate::maxima::protocol;
use crate::maxima::types::EvalResult;
use crate::maxima::unicode::unicode_to_maxima;
use crate::notebook::{CellOutput, CellStatus, CellType};
use crate::registry::NotebookContextRef;

/// Result of evaluating a cell, including all notebook effects produced.
pub struct EvalCellResult {
    pub eval_result: EvalResult,
    pub cell_output: CellOutput,
    pub execution_count: u32,
    /// Effects produced by notebook commands (Running status, Output/Error).
    /// Callers should iterate these to emit transport-specific notifications.
    pub effects: Vec<CommandEffect>,
}

/// Shared cell evaluation logic used by both Tauri and MCP transports.
///
/// Handles the full lifecycle: validate → set Running → prepare labels →
/// evaluate via Maxima → apply output. Consolidates lock acquisitions from
/// 6-7 down to 2 notebook locks + 1 session lock.
///
/// Returns `Err(AppError::CellIsMarkdown)` for markdown cells and
/// `Err(AppError::EmptyInput)` for blank cells so callers can decide
/// how to present these (error vs success-with-message).
pub async fn evaluate_cell(
    ctx: &NotebookContextRef,
    cell_id: &str,
    catalog: &Catalog,
    packages: &PackageCatalog,
    eval_timeout: u64,
) -> Result<EvalCellResult, AppError> {
    // ── Lock 1: validate, set Running, get exec count + label context ──
    let (input, exec_count, label_ctx) = {
        let mut nb = ctx.notebook.lock().await;
        let cell = nb
            .get_cell(cell_id)
            .ok_or_else(|| AppError::CellNotFound(cell_id.to_string()))?;

        if cell.cell_type == CellType::Markdown {
            return Err(AppError::CellIsMarkdown);
        }

        let input = cell.input.clone();
        if input.trim().is_empty() {
            return Err(AppError::EmptyInput);
        }

        // Set status to Running
        let _effect = nb.apply(NotebookCommand::SetCellStatus {
            cell_id: cell_id.to_string(),
            status: CellStatus::Running,
        });

        let ec = nb.next_execution_count();
        let lctx = LabelContext {
            label_map: nb.label_map().clone(),
            previous_output_label: nb.previous_output_label(cell_id),
        };

        (input, ec, lctx)
    };
    // Lock 1 dropped here

    // ── No lock: translate and rewrite ──
    let translated = unicode_to_maxima(&input);
    let rewritten = rewrite_labels(&translated, &label_ctx);

    // ── Clear capture sink ──
    ctx.capture_sink.take_cell_output();

    // ── Session lock: evaluate ──
    let mut guard = ctx.session.lock().await;
    let process = match guard.try_begin_eval() {
        Ok(p) => p,
        Err(e) => {
            // Set error status before returning
            let mut nb = ctx.notebook.lock().await;
            let _ = nb.apply(NotebookCommand::SetCellStatus {
                cell_id: cell_id.to_string(),
                status: CellStatus::Error,
            });
            return Err(e);
        }
    };

    let result = protocol::evaluate_with_packages(
        process,
        cell_id,
        &rewritten,
        catalog,
        packages,
        eval_timeout,
    )
    .await;

    guard.end_eval();
    drop(guard);
    // Session lock dropped here

    // ── Capture raw output ──
    let raw_output = ctx.capture_sink.take_cell_output();

    // ── Lock 2: apply output or error status ──
    let mut effects = Vec::new();

    match result {
        Ok(eval_result) => {
            let cell_output = CellOutput::from_eval_result(&eval_result, exec_count);
            let mut nb = ctx.notebook.lock().await;

            if let Some(ref label) = eval_result.output_label {
                nb.record_label(exec_count, label.clone());
            }

            if let Ok(effect) = nb.apply(NotebookCommand::SetCellOutput {
                cell_id: cell_id.to_string(),
                output: cell_output.clone(),
                raw_output,
            }) {
                effects.push(effect);
            }

            Ok(EvalCellResult {
                eval_result,
                cell_output,
                execution_count: exec_count,
                effects,
            })
        }
        Err(e) => {
            let mut nb = ctx.notebook.lock().await;
            let _ = nb.apply(NotebookCommand::SetCellStatus {
                cell_id: cell_id.to_string(),
                status: CellStatus::Error,
            });
            // Store raw output even on error
            if let Some(cell) = nb.get_cell_mut(cell_id) {
                cell.raw_output = raw_output;
            }
            Err(e)
        }
    }
}
