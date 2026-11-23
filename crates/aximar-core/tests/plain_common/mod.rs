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
        .unwrap()
        .cell_output
}

/// Run cells in a fresh session and check their one-line forms.
pub async fn check_plain_forms() {
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

    let out = run(&ctx, &catalog, &packages, "x^2/2;").await;
    assert_eq!(out.text_plain.as_deref(), Some("x^2/2"), "{out:?}");
    assert!(out.latex.is_some());

    // A long result stays one line
    let out = run(&ctx, &catalog, &packages, "expand((x+y)^30);").await;
    let plain = out.text_plain.expect("a one-line form");
    assert!(
        plain.starts_with("y^30+30*x*y^29") && !plain.contains('\n'),
        "{plain}"
    );

    // A string keeps its quotes.  The legacy path shows no value for a
    // string and the kernel-events path does, so check only that the
    // one-line form comes with a shown value
    let out = run(&ctx, &catalog, &packages, r#""a ~ \"quoted\" string";"#).await;
    let expected = out.latex.as_ref().map(|_| r#""a ~ \"quoted\" string""#);
    assert_eq!(out.text_plain.as_deref(), expected, "{out:?}");

    // No shown value: no one-line form
    let out = run(&ctx, &catalog, &packages, "x: 5$").await;
    assert_eq!(out.text_plain, None, "{out:?}");
    let out = run(&ctx, &catalog, &packages, "1/0;").await;
    assert_eq!(out.text_plain, None, "{out:?}");
}
