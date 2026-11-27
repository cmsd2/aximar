use std::sync::Arc;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::{NoEffects, NotebookCommand};
use aximar_core::evaluation::evaluate_cell;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::OutputSink;
use aximar_core::notebook::{CellOutput, CellType};
use aximar_core::registry::{NotebookContextRef, NotebookRegistry};
use aximar_core::session_ops::{ensure_session, SessionSettings};

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

/// Lisp errors keep their whole condition, error text leaves out the
/// debugger hint, and the session carries on afterwards.
pub async fn check_error_text() {
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

    // Wrong type
    let out = run(&ctx, &catalog, &packages, "?car(1);").await;
    assert!(out.is_error, "{out:?}");
    let error = out.error.as_deref().unwrap();
    assert!(error.contains("The value 1 is not of type LIST"), "{error}");
    assert!(!error.contains("Automatically continuing"), "{error}");
    assert!(!error.contains('\n'), "{error}");
    let info = out.error_info.as_ref().expect("an explanation");
    assert_eq!(info.id, "error.lisp_type");
    assert_eq!(info.explanation.params["value"], "1");
    assert_eq!(info.explanation.params["lisp_type"], "list");

    // Runaway recursion, then a normal cell
    let out = run(&ctx, &catalog, &packages, "g(n) := g(n + 1)$ g(1);").await;
    assert!(out.is_error, "{out:?}");
    assert_eq!(
        out.error_info.as_ref().expect("an explanation").id,
        "error.recursion"
    );
    let out = run(&ctx, &catalog, &packages, "2+2;").await;
    assert_eq!(out.text_plain.as_deref(), Some("4"), "{out:?}");

    // No debugger hint
    let out = run(&ctx, &catalog, &packages, "integrate(1/x, x, 0, 1);").await;
    assert!(out.is_error, "{out:?}");
    assert_eq!(out.error.as_deref(), Some("defint: integral is divergent."));
}
