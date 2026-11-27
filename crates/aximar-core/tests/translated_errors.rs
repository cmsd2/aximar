//! Maxima's error messages in the user's language (ax_messages.lisp), on
//! the legacy path: from session start, and after switching language with
//! a session running.
//!
//! Run with: `cargo test -p aximar-core --test translated_errors -- --ignored`

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

struct Session {
    registry: NotebookRegistry,
    ctx: NotebookContextRef,
    catalog: Catalog,
    packages: PackageCatalog,
}

impl Session {
    /// A notebook whose Maxima starts with `language`
    async fn start(language: Option<&str>) -> Self {
        let mut registry = NotebookRegistry::new();
        registry.set_language(language.map(str::to_string));
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
        Session {
            registry,
            ctx,
            catalog,
            packages: PackageCatalog::load(),
        }
    }

    /// Add a code cell holding `input` and run it
    async fn run(&self, input: &str) -> CellOutput {
        let cell_id = {
            let mut nb = self.ctx.notebook.lock().await;
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
        evaluate_cell(
            &self.ctx,
            &cell_id,
            &self.catalog,
            &self.packages,
            30,
            &NoEffects,
        )
        .await
        .unwrap()
        .cell_output
    }
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn failed_cells_carry_the_message_in_spanish() {
    let s = Session::start(Some("es")).await;

    // The English message stays, so Aximar still sees the error
    let out = s.run("1/0;").await;
    assert!(out.is_error, "{out:?}");
    assert!(
        out.error
            .as_deref()
            .unwrap()
            .contains("0 to a negative exponent"),
        "{out:?}"
    );
    assert_eq!(
        out.error_translated.as_deref(),
        Some("expt: no definido: desde 0 a un exponente negativo."),
        "{out:?}"
    );

    // Arguments are formatted into the translated pattern
    let out = s.run("diff();").await;
    assert_eq!(
        out.error_translated.as_deref(),
        Some("diff: número de argumentos equivocados."),
        "{out:?}"
    );

    // A message the catalog doesn't translate has no Spanish copy
    let out = s.run("error(\"my own message\");").await;
    assert!(out.is_error, "{out:?}");
    assert_eq!(out.error_translated, None, "{out:?}");

    // A caught error leaves no translation on a cell that succeeded
    let out = s.run("errcatch(1/0);").await;
    assert!(!out.is_error, "{out:?}");
    assert_eq!(out.error_translated, None, "{out:?}");

    // Evaluation carries on normally afterwards
    let out = s.run("2+2;").await;
    assert_eq!(out.latex.as_deref(), Some("4"), "{out:?}");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn maxima_errors_follow_a_switch() {
    let s = Session::start(None).await;

    // English: no translation
    let out = s.run("diff();").await;
    assert!(out.is_error, "{out:?}");
    assert_eq!(out.error_translated, None, "{out:?}");

    // Switching leaves the session, its variables and % alone
    s.run("x: 42$").await;
    s.run("[x, 6*7];").await;
    s.registry.set_language(Some("es".to_string()));
    let out = s.run("%;").await;
    assert_eq!(out.text_plain.as_deref(), Some("[42,42]"), "{out:?}");
    assert_eq!(out.text_output, "", "{out:?}");

    let out = s.run("diff();").await;
    assert_eq!(
        out.error_translated.as_deref(),
        Some("diff: número de argumentos equivocados."),
        "{out:?}"
    );

    // And back
    s.registry.set_language(Some("en".to_string()));
    let out = s.run("diff();").await;
    assert!(out.is_error, "{out:?}");
    assert_eq!(out.error_translated, None, "{out:?}");
}
