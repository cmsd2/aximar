//! Each kind of failed cell gets its explanation's ID and parameters,
//! against a real Maxima on the legacy path.
//!
//! Run with: `cargo test -p aximar-core --test error_ids -- --ignored`

use std::sync::Arc;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::{NoEffects, NotebookCommand};
use aximar_core::evaluation::evaluate_cell;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::OutputSink;
use aximar_core::maxima::types::ErrorInfo;
use aximar_core::notebook::CellType;
use aximar_core::registry::{NotebookContextRef, NotebookRegistry};
use aximar_core::session_ops::{ensure_session, SessionSettings};

/// Add a code cell holding `input`, run it, and return its explanation.
async fn explain(
    ctx: &NotebookContextRef,
    catalog: &Catalog,
    packages: &PackageCatalog,
    input: &str,
) -> ErrorInfo {
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
    let out = evaluate_cell(ctx, &cell_id, catalog, packages, 30, &NoEffects)
        .await
        .unwrap()
        .cell_output;
    assert!(out.is_error, "{input} should fail: {out:?}");
    out.error_info
        .unwrap_or_else(|| panic!("{input} has no explanation: {:?}", out.error))
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn failed_cells_carry_their_ids() {
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

    let info = explain(&ctx, &catalog, &packages, "1/0;").await;
    assert_eq!(info.id, "error.division_by_zero");
    assert_eq!(info.title.text, "Division by Zero");

    // A missing parenthesis
    let info = explain(&ctx, &catalog, &packages, "f(x;").await;
    assert_eq!(info.id, "error.syntax_missing");
    assert_eq!(info.explanation.params["delimiter"], ")");

    // Input that ends too early
    let info = explain(&ctx, &catalog, &packages, "1+;").await;
    assert_eq!(info.id, "error.incomplete_input");

    let info = explain(&ctx, &catalog, &packages, "x y;").await;
    assert_eq!(info.id, "error.syntax_operator");
    assert_eq!(info.explanation.params["token"], "y");

    // Parameters fill the text
    let info = explain(&ctx, &catalog, &packages, "f(x) := x$ f(1, 2);").await;
    assert_eq!(info.id, "error.arg_count");
    assert_eq!(info.explanation.params["func"], "f");
    assert_eq!(info.explanation.params["definition"], "f(x)");
    assert_eq!(info.explanation.params["found"], "[1,2]");
    assert!(
        info.explanation.text.contains("f(x)"),
        "{}",
        info.explanation
    );

    let info = explain(&ctx, &catalog, &packages, "integrate(1/x, x, 0, 1);").await;
    assert_eq!(info.id, "error.divergent_integral");
}
