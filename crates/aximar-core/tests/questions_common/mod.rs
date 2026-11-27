use std::sync::Arc;
use std::time::{Duration, Instant};

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::{NoEffects, NotebookCommand};
use aximar_core::evaluation::evaluate_cell;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::OutputSink;
use aximar_core::notebook::{CellOutput, CellType};
use aximar_core::registry::{NotebookContextRef, NotebookRegistry};
use aximar_core::session_ops::{ensure_session, SessionSettings};

/// Add a code cell holding `input` and run it.
async fn run(
    ctx: &NotebookContextRef,
    catalog: &Catalog,
    packages: &PackageCatalog,
    input: &str,
) -> CellOutput {
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
    evaluate_cell(ctx, &cell_id, catalog, packages, 30, &NoEffects)
        .await
        .unwrap_or_else(|e| panic!("{input} failed to run: {e}"))
        .cell_output
}

/// Questions stop their cell with an error that says how to answer them,
/// leave nothing behind for the next cell, and errcatch catches them.
pub async fn check_questions() {
    let mut registry = NotebookRegistry::new();
    let id = registry.create();
    let ctx = registry.get(&id).unwrap();
    let (catalog, packages) = (Catalog::load(), PackageCatalog::load());
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
        &catalog,
        None,
    )
    .await
    .expect("Maxima starts");

    // A sign question
    let start = Instant::now();
    let out = run(
        &ctx,
        &catalog,
        &packages,
        "integrate(exp(-a*x), x, 0, inf);",
    )
    .await;
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "took {:?}",
        start.elapsed()
    );
    assert!(out.is_error, "{out:?}");
    let error = out.error.as_deref().unwrap();
    assert!(
        error.contains("Is a positive, negative or zero?"),
        "{error}"
    );
    let info = out.error_info.as_ref().expect("an explanation");
    assert_eq!(info.id, "question.sign");
    assert!(info
        .suggestion
        .as_ref()
        .unwrap()
        .text
        .contains("assume(a > 0)"));

    // A yes/no question, well before the timeout
    let start = Instant::now();
    let out = run(&ctx, &catalog, &packages, "integrate(x^a, x);").await;
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "took {:?}",
        start.elapsed()
    );
    assert!(out.is_error, "{out:?}");
    let info = out.error_info.as_ref().expect("an explanation");
    assert_eq!(info.id, "question.equal");
    assert_eq!(info.explanation.params["a"], "a");
    assert_eq!(info.explanation.params["b"], "-1");

    // Nothing leaks into the next cell
    let out = run(&ctx, &catalog, &packages, "1/0;").await;
    let error = out.error.as_deref().unwrap();
    assert!(error.contains("0 to a negative exponent"), "{error}");
    assert!(!error.contains("positive"), "{error}");
    assert!(!out.text_output.contains("positive"), "{out:?}");

    // errcatch catches a question like any error
    let out = run(
        &ctx,
        &catalog,
        &packages,
        "errcatch(integrate(exp(-a*x), x, 0, inf));",
    )
    .await;
    assert!(!out.is_error, "{out:?}");
    assert_eq!(out.text_plain.as_deref(), Some("[]"), "{out:?}");

    // The session carries on, and an assumption answers the question
    let out = run(
        &ctx,
        &catalog,
        &packages,
        "assume(a > 0)$ integrate(exp(-a*x), x, 0, inf);",
    )
    .await;
    assert!(!out.is_error, "{out:?}");
    assert_eq!(out.text_plain.as_deref(), Some("1/a"), "{out:?}");
}
