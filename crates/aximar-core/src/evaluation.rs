use std::sync::atomic::Ordering;

use serde::Serialize;

use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::commands::{CommandEffect, EffectSink, NotebookCommand};
use crate::error::AppError;
use crate::maxima::labels::{rewrite_labels, LabelContext, UnresolvedLabel};
use crate::maxima::protocol;
use crate::maxima::types::EvalResult;
use crate::maxima::unicode::{unicode_to_maxima, UnsupportedCharacter};
use crate::notebook::{CellOutput, CellStatus, CellType, Notebook};
use crate::registry::NotebookContextRef;
use crate::safety::{check_trust, TrustDecision, TrustMode};

/// Result of evaluating a cell.
pub struct EvalCellResult {
    pub eval_result: EvalResult,
    pub cell_output: CellOutput,
    pub execution_count: u32,
}

/// Input that can't be sent to Maxima: a character Maxima code can't use,
/// or a label with no value in this session.  The message is shown to the
/// user as the cell's or expression's error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputError(pub String);

impl From<UnsupportedCharacter> for InputError {
    fn from(e: UnsupportedCharacter) -> Self {
        InputError(e.0)
    }
}

impl From<UnresolvedLabel> for InputError {
    fn from(e: UnresolvedLabel) -> Self {
        InputError(e.0)
    }
}

/// Prepare input for Maxima, the same way for cells and expressions from
/// either front end: Unicode is translated as `docs/unicode-input.md` sets
/// out (`α` becomes `alpha`), then label references such as `%o4` become
/// the session's labels.
pub fn prepare_input(input: &str, labels: &LabelContext) -> Result<String, InputError> {
    let translated = unicode_to_maxima(input)?;
    Ok(rewrite_labels(&translated, labels)?)
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

        match cell.cell_type {
            CellType::Markdown => return Err(AppError::CellIsMarkdown),
            CellType::Raw => return Err(AppError::CellIsRaw),
            CellType::Code => {}
        }

        let input = cell.input.clone();
        if input.trim().is_empty() {
            return Err(AppError::EmptyInput);
        }

        set_status(&mut nb, ctx, cell_id, CellStatus::Running, effects);

        let ec = nb.next_execution_count();
        (input, ec, nb.cell_label_context(cell_id))
    };
    // Lock 1 dropped here

    // ── No lock: translate and rewrite ──
    let rewritten = match prepare_input(&input, &label_ctx) {
        Ok(rewritten) => rewritten,
        // A rejected character or a label with no value: show the message
        // as the cell's error instead of letting Maxima misread the input
        Err(InputError(message)) => {
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
    // Only code cells run, so only they need trust
    if cell.cell_type != CellType::Code {
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

/// How one code cell fared in a run of all cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CellRunStatus {
    /// Evaluated without error.
    Ok,
    /// Nothing to execute (blank input).
    Empty,
    /// Evaluated, but Maxima reported an error.
    Error,
    /// Not run: it calls dangerous functions the trust rule blocks.
    NeedsTrust,
    /// Couldn't run (session failure, timeout, cancellation, ...).
    Failed,
    /// Not attempted because the run stopped earlier.
    NotRun,
}

impl CellRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CellRunStatus::Ok => "ok",
            CellRunStatus::Empty => "empty",
            CellRunStatus::Error => "error",
            CellRunStatus::NeedsTrust => "needs_trust",
            CellRunStatus::Failed => "failed",
            CellRunStatus::NotRun => "not_run",
        }
    }

    /// Whether this outcome stops the run.
    pub fn stops_run(self) -> bool {
        matches!(self, CellRunStatus::Error | CellRunStatus::NeedsTrust | CellRunStatus::Failed)
    }
}

/// One code cell's outcome in a run of all cells.
#[derive(Debug, Clone, Serialize)]
pub struct CellRun {
    pub cell_id: String,
    pub status: CellRunStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_count: Option<u32>,
    /// For `error` and `failed`: what went wrong.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// For `needs_trust`: the functions that blocked the cell.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dangerous_functions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl CellRun {
    /// An outcome with no details yet.
    pub fn new(cell_id: &str, status: CellRunStatus) -> Self {
        CellRun {
            cell_id: cell_id.to_string(),
            status,
            execution_count: None,
            error: None,
            dangerous_functions: Vec::new(),
            duration_ms: None,
        }
    }
}

/// Why a run of all cells stopped early.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    Error,
    NeedsTrust,
    Failed,
    /// The user stopped the run between cells.
    Cancelled,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            StopReason::Error => "error",
            StopReason::NeedsTrust => "needs_trust",
            StopReason::Failed => "failed",
            StopReason::Cancelled => "cancelled",
        }
    }
}

/// The outcome of a run of all cells: one entry per code cell, in order.
#[derive(Debug, Clone, Serialize)]
pub struct RunAllReport {
    pub cells: Vec<CellRun>,
    /// The cell that stopped the run; for a cancel, the first cell not run.
    pub stopped_at: Option<String>,
    pub stop_reason: Option<StopReason>,
}

impl RunAllReport {
    pub fn success(&self) -> bool {
        self.stop_reason.is_none()
    }
}

/// What a run of all cells needs besides the notebook.
pub struct RunAllOptions<'a> {
    pub catalog: &'a Catalog,
    pub packages: &'a PackageCatalog,
    pub eval_timeout: u64,
    pub mode: TrustMode,
    pub effects: &'a dyn EffectSink,
}

/// Run every code cell in order, for the window's Run All, MCP's
/// `run_all_cells` and batch runs.  Stops at the first cell that ends in an
/// error, is blocked by the trust check or fails to run, and marks the cells
/// after it `not_run`.  Blank cells are skipped.  Between cells it checks the
/// notebook's run-all cancel flag (`NotebookContextRef::cancel_run_all`).
///
/// The session must already be running: starting it differs by front end.
/// `on_cell` hears each cell's outcome as it finishes, for progress output.
pub async fn run_all(
    ctx: &NotebookContextRef,
    opts: &RunAllOptions<'_>,
    on_cell: &mut (dyn FnMut(&CellRun) + Send),
) -> RunAllReport {
    ctx.run_all_cancel.store(false, Ordering::SeqCst);
    let cell_ids: Vec<String> = {
        let nb = ctx.notebook.lock().await;
        nb.cells()
            .iter()
            .filter(|c| c.cell_type == CellType::Code)
            .map(|c| c.id.clone())
            .collect()
    };

    let mut cells = Vec::with_capacity(cell_ids.len());
    let mut stop: Option<(String, StopReason)> = None;
    for cell_id in &cell_ids {
        if stop.is_none() && ctx.run_all_cancel.swap(false, Ordering::SeqCst) {
            stop = Some((cell_id.clone(), StopReason::Cancelled));
        }
        if stop.is_some() {
            cells.push(CellRun::new(cell_id, CellRunStatus::NotRun));
            continue;
        }
        let run = run_one(ctx, cell_id, opts).await;
        on_cell(&run);
        if run.status.stops_run() {
            // A cancel during the cell usually ends it in a failure or an
            // error; report the cancel, which is why it ended
            let reason = if ctx.run_all_cancel.swap(false, Ordering::SeqCst) {
                StopReason::Cancelled
            } else {
                match run.status {
                    CellRunStatus::Error => StopReason::Error,
                    CellRunStatus::NeedsTrust => StopReason::NeedsTrust,
                    _ => StopReason::Failed,
                }
            };
            stop = Some((cell_id.clone(), reason));
        }
        cells.push(run);
    }

    let (stopped_at, stop_reason) = match stop {
        Some((cell_id, reason)) => (Some(cell_id), Some(reason)),
        None => (None, None),
    };
    RunAllReport { cells, stopped_at, stop_reason }
}

/// Check and run one cell of a run of all cells.
async fn run_one(ctx: &NotebookContextRef, cell_id: &str, opts: &RunAllOptions<'_>) -> CellRun {
    match check_cell_trust(ctx, cell_id, opts.packages, opts.mode, opts.effects).await {
        Ok(TrustDecision::Allowed) => {}
        Ok(TrustDecision::NeedsTrust { functions }) => {
            let mut run = CellRun::new(cell_id, CellRunStatus::NeedsTrust);
            run.dangerous_functions = functions;
            return run;
        }
        Err(e) => {
            let mut run = CellRun::new(cell_id, CellRunStatus::Failed);
            run.error = Some(e.to_string());
            return run;
        }
    }

    let result = evaluate_cell(
        ctx,
        cell_id,
        opts.catalog,
        opts.packages,
        opts.eval_timeout,
        opts.effects,
    )
    .await;
    match result {
        Ok(result) => {
            let output = &result.cell_output;
            let status = if output.is_error { CellRunStatus::Error } else { CellRunStatus::Ok };
            let mut run = CellRun::new(cell_id, status);
            run.execution_count = Some(result.execution_count);
            run.duration_ms = Some(output.duration_ms);
            if output.is_error {
                // Maxima's error text usually lands in the text output; `error`
                // is set for timeouts and label errors
                let text = output
                    .error
                    .as_deref()
                    .filter(|e| !e.is_empty())
                    .unwrap_or(&output.text_output);
                run.error = Some(text.trim().to_string());
            }
            run
        }
        Err(AppError::EmptyInput) => CellRun::new(cell_id, CellRunStatus::Empty),
        Err(e) => {
            let mut run = CellRun::new(cell_id, CellRunStatus::Failed);
            run.error = Some(e.to_string());
            run
        }
    }
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

    // ── Input preparation ──

    #[tokio::test]
    async fn cells_and_expressions_are_prepared_alike() {
        let (ctx, cell_id) = notebook_with("α + 1;", CellType::Code).await;
        let nb = ctx.notebook.lock().await;
        for labels in [nb.cell_label_context(&cell_id), nb.expression_label_context()] {
            assert_eq!(prepare_input("α + 1;", &labels), Ok("alpha + 1;".to_string()));
            let err = prepare_input("%o9 + 1;", &labels).unwrap_err();
            assert!(err.0.contains("Out[9]"), "{err:?}");
        }
    }

    #[tokio::test]
    async fn bare_percent_means_the_cell_above_only_in_cells() {
        let (ctx, cell_id) = notebook_with("% + 1;", CellType::Code).await;
        let nb = ctx.notebook.lock().await;
        // No cell above has an output, so a cell can't use %
        assert!(prepare_input("% + 1;", &nb.cell_label_context(&cell_id)).is_err());
        // An expression's % is Maxima's last result
        assert_eq!(
            prepare_input("% + 1;", &nb.expression_label_context()),
            Ok("% + 1;".to_string())
        );
    }

    #[tokio::test]
    async fn rejected_character_fails_the_cell_before_maxima_runs() {
        // No session was started: reaching Maxima would fail differently
        let sink = Recorder::default();
        let (ctx, cell_id) = notebook_with("a ≈ b;", CellType::Code).await;
        let result = run(&ctx, &cell_id, &sink).await.unwrap();
        assert!(result.cell_output.is_error);
        let error = result.cell_output.error.unwrap();
        assert!(error.contains("U+2248") && error.contains("abs(a - b)"), "{error}");
        assert_eq!(
            *sink.0.lock().unwrap(),
            vec!["cell_status_updated Running", "cell_output_updated Error"]
        );
    }

    #[tokio::test]
    async fn labels_on_loaded_cells_have_no_value() {
        // A notebook from a file shows [3] on a cell, but no run in this
        // session made it: %o3 and a bare % must not read whatever Maxima's
        // own %o3 holds now (#8)
        let (ctx, _) = notebook_with("", CellType::Code).await;
        let mut nb = ctx.notebook.lock().await;
        let output = CellOutput {
            text_output: String::new(),
            latex: Some("42".into()),
            plot_svg: None,
            plot_data: None,
            image_png: None,
            error: None,
            is_error: false,
            duration_ms: 0,
            output_label: Some("%o7".into()),
            execution_count: Some(3),
            ..Default::default()
        };
        nb.apply(NotebookCommand::LoadCells {
            cells: vec![
                ("loaded".into(), CellType::Code, "6 * 7;".into(), Some(output)),
                ("next".into(), CellType::Code, String::new(), None),
            ],
        })
        .unwrap();
        let labels = nb.cell_label_context("next");
        let err = prepare_input("%o3 + 1;", &labels).unwrap_err();
        assert!(err.0.contains("Out[3] (%o3) has no value in this session"), "{err:?}");
        assert!(prepare_input("% + 1;", &labels).is_err());
        // New runs number from 1 again, as in a fresh session
        assert_eq!(nb.next_execution_count(), 1);
    }

    #[tokio::test]
    async fn a_raw_cell_is_never_run() {
        let sink = Recorder::default();
        let (ctx, _) = notebook_with("", CellType::Code).await;
        let text = r#"{"nbformat": 4, "nbformat_minor": 5, "metadata": {}, "cells": [
            {"id": "r", "cell_type": "raw", "source": "system(\"ls\");", "metadata": {}}]}"#;
        ctx.notebook
            .lock()
            .await
            .apply(NotebookCommand::LoadNotebook { file: crate::notebooks::format::read(text).unwrap().into() })
            .unwrap();
        assert!(matches!(run(&ctx, "r", &sink).await, Err(AppError::CellIsRaw)));
        // Nor does it ask for trust, whatever its text
        let decision = check_cell_trust(&ctx, "r", &PackageCatalog::load(), TrustMode::Interactive, &sink).await.unwrap();
        assert!(matches!(decision, TrustDecision::Allowed));
        assert!(sink.0.lock().unwrap().is_empty());
    }
}
