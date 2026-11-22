use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::commands::{CommandEffect, EffectSink, NotebookCommand};
use crate::error::AppError;
use crate::maxima::labels::{rewrite_labels, LabelContext, UnresolvedLabel};
use crate::maxima::protocol;
use crate::maxima::types::EvalResult;
use crate::maxima::unicode::unicode_to_maxima;
use crate::notebook::{CellOutput, CellStatus, CellType, Notebook};
use crate::registry::NotebookContextRef;
use crate::safety::{check_trust, TrustDecision, TrustMode};

/// Result of evaluating a cell.
pub struct EvalCellResult {
    pub eval_result: EvalResult,
    pub cell_output: CellOutput,
    pub execution_count: u32,
}

/// Shared cell evaluation logic used by both Tauri and MCP transports.
///
/// Handles the full lifecycle: validate → set Running → prepare labels →
/// evaluate via Maxima → apply output. Consolidates lock acquisitions from
/// 6-7 down to 2 notebook locks + 1 session lock.
///
/// Every change to the cell is reported to `effects` as it happens: Running
/// when evaluation starts, then the output, or Error when evaluation fails.
///
/// Returns `Err(AppError::CellIsMarkdown)` for markdown cells and
/// `Err(AppError::EmptyInput)` for blank cells so callers can decide
/// how to present these (error vs success-with-message).  Neither changes
/// the cell or reports an effect.
pub async fn evaluate_cell(
    ctx: &NotebookContextRef,
    cell_id: &str,
    catalog: &Catalog,
    packages: &PackageCatalog,
    eval_timeout: u64,
    effects: &dyn EffectSink,
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

        set_status(&mut nb, ctx, cell_id, CellStatus::Running, effects);

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
    let rewritten = match rewrite_labels(&translated, &label_ctx) {
        Ok(rewritten) => rewritten,
        // A label with no value: show the message as the cell's error
        // instead of letting Maxima read an unrelated label of its own
        Err(UnresolvedLabel(message)) => {
            return Ok(label_error_result(ctx, cell_id, exec_count, message, effects).await);
        }
    };

    // ── Session lock: evaluate ──
    let mut guard = ctx.session.lock().await;
    let process = match guard.try_begin_eval() {
        Ok(p) => p,
        Err(e) => {
            let mut nb = ctx.notebook.lock().await;
            set_status(&mut nb, ctx, cell_id, CellStatus::Error, effects);
            return Err(e);
        }
    };

    // Clear and read the capture sink only while holding the session lock.
    // A run queued behind this one would otherwise clear this cell's output
    // while it is still being written, or before it is read.
    ctx.capture_sink.take_cell_output();

    let result = protocol::evaluate_with_packages(
        process,
        cell_id,
        &rewritten,
        catalog,
        packages,
        eval_timeout,
    )
    .await;

    let raw_output = ctx.capture_sink.take_cell_output();

    guard.end_eval();
    drop(guard);
    // Session lock dropped here

    // ── Lock 2: apply output or error status ──
    let mut nb = ctx.notebook.lock().await;
    match result {
        Ok(eval_result) => {
            let cell_output = CellOutput::from_eval_result(&eval_result, exec_count);

            if let Some(ref label) = eval_result.output_label {
                nb.record_label(exec_count, label.clone(), cell_id);
            }

            if let Ok(effect) = nb.apply(NotebookCommand::SetCellOutput {
                cell_id: cell_id.to_string(),
                output: cell_output.clone(),
                raw_output,
            }) {
                effects.effect(&ctx.id, &nb, &effect);
            }

            Ok(EvalCellResult {
                eval_result,
                cell_output,
                execution_count: exec_count,
            })
        }
        Err(e) => {
            // Store raw output even on error, before reporting the status
            if let Some(cell) = nb.get_cell_mut(cell_id) {
                cell.raw_output = raw_output;
            }
            set_status(&mut nb, ctx, cell_id, CellStatus::Error, effects);
            Err(e)
        }
    }
}

/// Check a cell against the trust rule before running it.  When an
/// interactive run is blocked, report `TrustRequired` so the window shows its
/// trust banner, whichever front end ran the cell.  Markdown cells don't run,
/// so they are always allowed.
pub async fn check_cell_trust(
    ctx: &NotebookContextRef,
    cell_id: &str,
    packages: &PackageCatalog,
    mode: TrustMode,
    effects: &dyn EffectSink,
) -> Result<TrustDecision, AppError> {
    let nb = ctx.notebook.lock().await;
    let cell = nb
        .get_cell(cell_id)
        .ok_or_else(|| AppError::CellNotFound(cell_id.to_string()))?;
    if cell.cell_type == CellType::Markdown {
        return Ok(TrustDecision::Allowed);
    }
    let decision = check_trust(&nb, &cell.input, packages, mode);
    if let (TrustMode::Interactive, TrustDecision::NeedsTrust { functions }) = (mode, &decision) {
        let effect = CommandEffect::TrustRequired {
            cell_id: cell_id.to_string(),
            functions: functions.clone(),
        };
        effects.effect(&ctx.id, &nb, &effect);
    }
    Ok(decision)
}

/// Set a cell's status and report the change.
fn set_status(
    nb: &mut Notebook,
    ctx: &NotebookContextRef,
    cell_id: &str,
    status: CellStatus,
    effects: &dyn EffectSink,
) {
    if let Ok(effect) = nb.apply(NotebookCommand::SetCellStatus {
        cell_id: cell_id.to_string(),
        status,
    }) {
        effects.effect(&ctx.id, nb, &effect);
    }
}

/// Finish a cell whose input refers to a label with no value, without
/// running Maxima: the message becomes the cell's error output.
async fn label_error_result(
    ctx: &NotebookContextRef,
    cell_id: &str,
    exec_count: u32,
    message: String,
    effects: &dyn EffectSink,
) -> EvalCellResult {
    let eval_result = EvalResult {
        cell_id: cell_id.to_string(),
        error: Some(message),
        is_error: true,
        ..Default::default()
    };
    let cell_output = CellOutput::from_eval_result(&eval_result, exec_count);
    let mut nb = ctx.notebook.lock().await;
    if let Ok(effect) = nb.apply(NotebookCommand::SetCellOutput {
        cell_id: cell_id.to_string(),
        output: cell_output.clone(),
        raw_output: Vec::new(),
    }) {
        effects.effect(&ctx.id, &nb, &effect);
    }
    EvalCellResult {
        eval_result,
        cell_output,
        execution_count: exec_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::NotebookRegistry;
    use std::sync::Mutex;

    /// Records each reported effect with the cell's status at that moment.
    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    impl EffectSink for Recorder {
        fn effect(&self, _notebook_id: &str, nb: &Notebook, effect: &CommandEffect) {
            let status = effect
                .cell_id()
                .and_then(|id| nb.get_cell(id))
                .map(|cell| format!("{:?}", cell.status))
                .unwrap_or_default();
            self.0.lock().unwrap().push(format!("{} {status}", effect.effect_name()));
        }
    }

    /// A notebook whose first cell holds `input`, with no Maxima session.
    async fn notebook_with(input: &str, cell_type: CellType) -> (NotebookContextRef, String) {
        let mut registry = NotebookRegistry::new();
        let id = registry.create();
        let ctx = registry.get(&id).unwrap();
        let cell_id = {
            let mut nb = ctx.notebook.lock().await;
            let cell_id = nb.cells()[0].id.clone();
            if cell_type == CellType::Markdown {
                nb.apply(NotebookCommand::ToggleCellType { cell_id: cell_id.clone() }).unwrap();
            }
            nb.apply(NotebookCommand::UpdateCellInput {
                cell_id: cell_id.clone(),
                input: input.to_string(),
            })
            .unwrap();
            cell_id
        };
        (ctx, cell_id)
    }

    async fn run(ctx: &NotebookContextRef, cell_id: &str, sink: &Recorder) -> Result<EvalCellResult, AppError> {
        evaluate_cell(ctx, cell_id, &Catalog::load(), &PackageCatalog::load(), 5, sink).await
    }

    #[tokio::test]
    async fn markdown_and_empty_cells_report_nothing() {
        let sink = Recorder::default();
        let (ctx, cell_id) = notebook_with("# Title", CellType::Markdown).await;
        assert!(matches!(run(&ctx, &cell_id, &sink).await, Err(AppError::CellIsMarkdown)));
        let (ctx, cell_id) = notebook_with("   ", CellType::Code).await;
        assert!(matches!(run(&ctx, &cell_id, &sink).await, Err(AppError::EmptyInput)));
        assert!(sink.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn failure_to_run_reports_running_then_error() {
        // No session was started, so the evaluation can't begin
        let sink = Recorder::default();
        let (ctx, cell_id) = notebook_with("2 + 3;", CellType::Code).await;
        assert!(matches!(run(&ctx, &cell_id, &sink).await, Err(AppError::ProcessNotRunning)));
        assert_eq!(
            *sink.0.lock().unwrap(),
            vec!["cell_status_updated Running", "cell_status_updated Error"]
        );
    }

    #[tokio::test]
    async fn label_error_reports_running_then_the_error_output() {
        let sink = Recorder::default();
        let (ctx, cell_id) = notebook_with("%o9 + 1;", CellType::Code).await;
        let result = run(&ctx, &cell_id, &sink).await.unwrap();
        assert!(result.cell_output.is_error);
        assert_eq!(
            *sink.0.lock().unwrap(),
            vec!["cell_status_updated Running", "cell_output_updated Error"]
        );
    }

    async fn trust(ctx: &NotebookContextRef, cell_id: &str, mode: TrustMode, sink: &Recorder) -> TrustDecision {
        check_cell_trust(ctx, cell_id, &PackageCatalog::load(), mode, sink).await.unwrap()
    }

    #[tokio::test]
    async fn blocked_interactive_run_asks_for_trust() {
        let sink = Recorder::default();
        let (ctx, cell_id) = notebook_with("load(\"helpers.mac\")$", CellType::Code).await;
        ctx.notebook.lock().await.set_trusted(false);
        let decision = trust(&ctx, &cell_id, TrustMode::Interactive, &sink).await;
        assert_eq!(decision, TrustDecision::NeedsTrust { functions: vec!["load".into()] });
        // The notebook is unchanged, so the cell keeps its status
        assert_eq!(*sink.0.lock().unwrap(), vec!["trust_required Idle"]);
    }

    #[tokio::test]
    async fn blocked_headless_run_reports_nothing() {
        let sink = Recorder::default();
        let (ctx, cell_id) = notebook_with("system(\"ls\")$", CellType::Code).await;
        let decision = trust(&ctx, &cell_id, TrustMode::Headless { allow_dangerous: false }, &sink).await;
        assert_eq!(decision, TrustDecision::NeedsTrust { functions: vec!["system".into()] });
        assert!(sink.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn markdown_mentioning_a_dangerous_call_is_allowed() {
        let sink = Recorder::default();
        let (ctx, cell_id) = notebook_with("Don't call system(\"rm\") here", CellType::Markdown).await;
        ctx.notebook.lock().await.set_trusted(false);
        assert_eq!(trust(&ctx, &cell_id, TrustMode::Interactive, &sink).await, TrustDecision::Allowed);
        assert!(sink.0.lock().unwrap().is_empty());
    }
}
