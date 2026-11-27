//! Maxima's notes, recognised in real output on the legacy path.
//!
//! Run with: `cargo test -p aximar-core --test notes -- --ignored`

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

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn notes_in_real_output() {
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

    // Decimals replaced: one note for both lines, and the result as before
    let out = run(&ctx, &catalog, &packages, "rat(0.5*x + 0.25);").await;
    assert!(!out.is_error, "{out:?}");
    assert_eq!(out.notes.len(), 1, "{out:?}");
    let note = &out.notes[0];
    assert_eq!(note.id, "note.rat_replaced");
    assert_eq!((note.start, note.end), (0, 2));
    let replacements = &note.message.params["replacements"];
    assert!(replacements.contains("0.25 → 1/4"), "{replacements}");
    assert!(replacements.contains("0.5 → 1/2"), "{replacements}");
    assert_eq!(out.text_plain.as_deref(), Some("(2*x+1)/4"));

    // Solutions may be lost: both of Maxima's lines
    let out = run(&ctx, &catalog, &packages, "solve(sin(x) = 1/2, x);").await;
    assert_eq!(out.notes.len(), 1, "{out:?}");
    assert_eq!(out.notes[0].id, "note.solve_arc_trig");
    assert_eq!((out.notes[0].start, out.notes[0].end), (0, 2));

    // A passing division by zero, in a cell that still has a result
    let out = run(&ctx, &catalog, &packages, "romberg(1/x, x, 0, 1);").await;
    assert!(!out.is_error, "{out:?}");
    assert_eq!(out.notes.len(), 1, "{out:?}");
    assert_eq!(out.notes[0].id, "note.zero_power");

    // Print text stays text
    let out = run(&ctx, &catalog, &packages, "print(\"rat: my own label\")$").await;
    assert!(out.notes.is_empty(), "{out:?}");
    assert_eq!(out.text_output, "rat: my own label");
}
