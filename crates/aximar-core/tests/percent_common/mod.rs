use std::sync::Arc;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::{NoEffects, NotebookCommand};
use aximar_core::evaluation::evaluate_cell;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::OutputSink;
use aximar_core::maxima::protocol;
use aximar_core::maxima::types::EvalResult;
use aximar_core::notebook::{CellOutput, CellType};
use aximar_core::registry::{NotebookContextRef, NotebookRegistry};
use aximar_core::session_ops::{ensure_session, SessionSettings};

/// Evaluate an expression with no cell, as `evaluate_expression` does.
async fn expr(ctx: &NotebookContextRef, catalog: &Catalog, input: &str) -> EvalResult {
    let mut guard = ctx.session.lock().await;
    let process = guard.try_begin_eval().unwrap();
    let result = protocol::evaluate(process, "expr", input, catalog, 30).await.unwrap();
    guard.end_eval();
    result
}

/// Add a code cell holding `input` and run it.
async fn cell(ctx: &NotebookContextRef, catalog: &Catalog, packages: &PackageCatalog, input: &str) -> CellOutput {
    let cell_id = {
        let mut nb = ctx.notebook.lock().await;
        let effect = nb
            .apply(NotebookCommand::AddCell {
                cell_type: CellType::Code,
                input: input.to_string(),
                after_cell_id: None,
                before_cell_id: None,
            })
            .unwrap();
        effect.cell_id().unwrap().to_string()
    };
    evaluate_cell(ctx, &cell_id, catalog, packages, 30, &NoEffects).await.unwrap().cell_output
}

/// Maxima's `%` holds the last result after each evaluation, as in plain
/// Maxima, whatever Aximar writes after the expression.
pub async fn check_percent() {
    let mut registry = NotebookRegistry::new();
    let id = registry.create();
    let ctx = registry.get(&id).unwrap();
    let (catalog, packages) = (Catalog::load(), PackageCatalog::load());
    let settings = SessionSettings { backend: Backend::Local, maxima_path: None, eval_timeout: 30 };
    ensure_session(&ctx, &settings, |ctx| ctx.capture_sink.clone() as Arc<dyn OutputSink>, |_| None, &catalog, None)
        .await
        .expect("Maxima starts");

    expr(&ctx, &catalog, "7;").await;
    let r = expr(&ctx, &catalog, "% + 1;").await;
    assert_eq!(r.latex.as_deref(), Some("8"), "{r:?}");

    // A silent result is still the last result
    expr(&ctx, &catalog, "x: 20$").await;
    let r = expr(&ctx, &catalog, "% + 1;").await;
    assert_eq!(r.latex.as_deref(), Some("21"), "{r:?}");

    // After an error, % keeps the result before it, as in plain Maxima
    let r = expr(&ctx, &catalog, "1/0;").await;
    assert!(r.is_error, "{r:?}");
    let r = expr(&ctx, &catalog, "%;").await;
    assert_eq!(r.latex.as_deref(), Some("21"), "{r:?}");

    // Output labels still name the expression's own result
    let r = expr(&ctx, &catalog, "100;").await;
    let label = r.output_label.clone().expect("a label");
    let r = expr(&ctx, &catalog, &format!("{label} + 1;")).await;
    assert_eq!(r.latex.as_deref(), Some("101"), "{r:?}");

    // Cells keep their own labels and bare %
    let a = cell(&ctx, &catalog, &packages, "3;").await;
    assert_eq!(a.latex.as_deref(), Some("3"));
    let b = cell(&ctx, &catalog, &packages, "% * 2;").await;
    assert_eq!(b.latex.as_deref(), Some("6"), "{b:?}");
    let c = cell(&ctx, &catalog, &packages, &format!("%o{} + %o{};", a.execution_count.unwrap(), b.execution_count.unwrap())).await;
    assert_eq!(c.latex.as_deref(), Some("9"), "{c:?}");
}
