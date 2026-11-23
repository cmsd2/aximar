//! Running all cells against a real Maxima.
//!
//! Marked `#[ignore]` so they don't run in CI by default.
//! Run with: `cargo test -p aximar-core --test run_all -- --ignored`

use std::sync::{Arc, Mutex};

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::{CommandEffect, EffectSink, NoEffects, NotebookCommand};
use aximar_core::evaluation::{
    evaluate_cell, run_all, CellRun, CellRunStatus, RunAllOptions, RunAllReport, StopReason,
};
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::OutputSink;
use aximar_core::notebook::{CellType, Notebook};
use aximar_core::registry::{NotebookContextRef, NotebookRegistry};
use aximar_core::safety::TrustMode;
use aximar_core::session_ops::{ensure_session, SessionSettings};

/// A notebook holding one code cell per input, with Maxima running.
async fn notebook_with(inputs: &[&str], catalog: &Catalog) -> (NotebookContextRef, Vec<String>) {
    let mut registry = NotebookRegistry::new();
    let id = registry.create();
    let ctx = registry.get(&id).unwrap();
    let cell_ids = {
        let mut nb = ctx.notebook.lock().await;
        // A new notebook has one empty cell, which can't be deleted: use it
        let first = nb.cells()[0].id.clone();
        nb.apply(NotebookCommand::UpdateCellInput {
            cell_id: first.clone(),
            input: inputs[0].into(),
        })
        .unwrap();
        let mut ids = vec![first];
        for input in &inputs[1..] {
            let effect = nb
                .apply(NotebookCommand::AddCell {
                    cell_type: CellType::Code,
                    input: input.to_string(),
                    after_cell_id: ids.last().cloned(),
                    before_cell_id: None,
                })
                .unwrap();
            ids.push(effect.cell_id().unwrap().to_string());
        }
        ids
    };
    let settings = SessionSettings {
        backend: Backend::Local,
        maxima_path: None,
        eval_timeout: 30,
    };
    ensure_session(
        &ctx,
        &settings,
        |ctx| ctx.capture_sink.clone() as Arc<dyn OutputSink>,
        |_| None,
        catalog,
        None,
    )
    .await
    .expect("Maxima starts");
    (ctx, cell_ids)
}

fn statuses(report: &RunAllReport) -> Vec<CellRunStatus> {
    report.cells.iter().map(|c| c.status).collect()
}

/// Records the names of the effects reported.
#[derive(Default)]
struct Recorder(Mutex<Vec<String>>);

impl EffectSink for Recorder {
    fn effect(&self, _notebook_id: &str, _nb: &Notebook, effect: &CommandEffect) {
        self.0
            .lock()
            .unwrap()
            .push(effect.effect_name().to_string());
    }
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn stops_at_an_error_in_the_middle() {
    let (catalog, packages) = (Catalog::load(), PackageCatalog::load());
    let (ctx, ids) = notebook_with(&["a: 1;", "   ", "1/0;", "b: 2;"], &catalog).await;
    let opts = RunAllOptions {
        catalog: &catalog,
        packages: &packages,
        eval_timeout: 30,
        mode: TrustMode::Interactive,
        effects: &NoEffects,
    };
    let report = run_all(&ctx, &opts, &mut |_| {}).await;

    use CellRunStatus::*;
    assert_eq!(statuses(&report), vec![Ok, Empty, Error, NotRun]);
    assert_eq!(report.stop_reason, Some(StopReason::Error));
    assert_eq!(report.stopped_at.as_deref(), Some(ids[2].as_str()));
    assert!(
        report.cells[2]
            .error
            .as_deref()
            .unwrap()
            .contains("0 to a negative exponent"),
        "{report:?}"
    );

    // The cell after the error never ran, so b is still unbound
    let check = {
        let mut nb = ctx.notebook.lock().await;
        let effect = nb
            .apply(NotebookCommand::AddCell {
                cell_type: CellType::Code,
                input: "b;".into(),
                after_cell_id: None,
                before_cell_id: None,
            })
            .unwrap();
        effect.cell_id().unwrap().to_string()
    };
    let result = evaluate_cell(&ctx, &check, &catalog, &packages, 30, &NoEffects)
        .await
        .unwrap();
    assert_eq!(
        result.cell_output.latex.as_deref(),
        Some("b"),
        "{:?}",
        result.cell_output
    );
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn stops_at_a_trust_block_and_asks_for_trust() {
    let (catalog, packages) = (Catalog::load(), PackageCatalog::load());
    let (ctx, ids) = notebook_with(&["a: 1;", "load(\"helpers.mac\");", "b: 2;"], &catalog).await;
    ctx.notebook.lock().await.set_trusted(false);
    let sink = Recorder::default();
    let opts = RunAllOptions {
        catalog: &catalog,
        packages: &packages,
        eval_timeout: 30,
        mode: TrustMode::Interactive,
        effects: &sink,
    };
    let report = run_all(&ctx, &opts, &mut |_| {}).await;

    use CellRunStatus::*;
    assert_eq!(statuses(&report), vec![Ok, NeedsTrust, NotRun]);
    assert_eq!(report.stop_reason, Some(StopReason::NeedsTrust));
    assert_eq!(report.stopped_at.as_deref(), Some(ids[1].as_str()));
    assert_eq!(
        report.cells[1].dangerous_functions,
        vec!["load".to_string()]
    );
    assert!(sink
        .0
        .lock()
        .unwrap()
        .contains(&"trust_required".to_string()));
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn cancel_stops_before_the_next_cell() {
    let (catalog, packages) = (Catalog::load(), PackageCatalog::load());
    let (ctx, ids) = notebook_with(&["a: 1;", "b: 2;", "c: 3;"], &catalog).await;
    let opts = RunAllOptions {
        catalog: &catalog,
        packages: &packages,
        eval_timeout: 30,
        mode: TrustMode::Interactive,
        effects: &NoEffects,
    };
    // The user presses stop while the first cell runs
    let cancel = ctx.clone();
    let mut seen = Vec::new();
    let report = run_all(&ctx, &opts, &mut |run: &CellRun| {
        seen.push(run.cell_id.clone());
        cancel.cancel_run_all();
    })
    .await;

    use CellRunStatus::*;
    assert_eq!(statuses(&report), vec![Ok, NotRun, NotRun]);
    assert_eq!(report.stop_reason, Some(StopReason::Cancelled));
    assert_eq!(report.stopped_at.as_deref(), Some(ids[1].as_str()));
    assert_eq!(seen, vec![ids[0].clone()]);
}
