//! Maxima's error messages in the user's language (ax_messages.lisp), on
//! the legacy path, with AXIMAR_LANGUAGE=es.  A separate binary because it
//! sets the variable before the session starts.
//!
//! Run with: `cargo test -p aximar-core --test translated_errors -- --ignored`

use std::sync::Arc;

use aximar_core::catalog::search::Catalog;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::OutputSink;
use aximar_core::maxima::protocol;
use aximar_core::maxima::types::EvalResult;
use aximar_core::registry::{NotebookContextRef, NotebookRegistry};
use aximar_core::session_ops::{ensure_session, SessionSettings};

async fn expr(ctx: &NotebookContextRef, catalog: &Catalog, input: &str) -> EvalResult {
    let mut guard = ctx.session.lock().await;
    let process = guard.try_begin_eval().unwrap();
    let result = protocol::evaluate(process, "expr", input, catalog, 30)
        .await
        .unwrap();
    guard.end_eval();
    result
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn failed_cells_carry_the_message_in_spanish() {
    // Read when the session starts; this binary runs only this test
    unsafe { std::env::set_var("AXIMAR_LANGUAGE", "es") };

    let mut registry = NotebookRegistry::new();
    let id = registry.create();
    let ctx = registry.get(&id).unwrap();
    let catalog = Catalog::load();
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

    // The English message stays, so Aximar still sees the error
    let r = expr(&ctx, &catalog, "1/0;").await;
    assert!(r.is_error, "{r:?}");
    assert!(
        r.error
            .as_deref()
            .unwrap()
            .contains("0 to a negative exponent"),
        "{r:?}"
    );
    assert_eq!(
        r.error_translated.as_deref(),
        Some("expt: no definido: desde 0 a un exponente negativo."),
        "{r:?}"
    );

    // Arguments are formatted into the translated pattern
    let r = expr(&ctx, &catalog, "diff();").await;
    assert_eq!(
        r.error_translated.as_deref(),
        Some("diff: número de argumentos equivocados."),
        "{r:?}"
    );

    // A message the catalog doesn't translate has no Spanish copy
    let r = expr(&ctx, &catalog, "error(\"my own message\");").await;
    assert!(r.is_error, "{r:?}");
    assert_eq!(r.error_translated, None, "{r:?}");

    // A caught error leaves no translation on a cell that succeeded
    let r = expr(&ctx, &catalog, "errcatch(1/0);").await;
    assert!(!r.is_error, "{r:?}");
    assert_eq!(r.error_translated, None, "{r:?}");

    // Evaluation carries on normally afterwards
    let r = expr(&ctx, &catalog, "2+2;").await;
    assert_eq!(r.latex.as_deref(), Some("4"), "{r:?}");
}
