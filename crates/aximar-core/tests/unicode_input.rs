//! Unicode input against a real Maxima: the `unicode-input` scenarios
//! that need one.
//!
//! Marked `#[ignore]` so they don't run in CI by default.
//! Run with: `cargo test -p aximar-core --test unicode_input -- --ignored`

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

/// A notebook with Maxima running, which runs cells one after another.
struct Session {
    ctx: NotebookContextRef,
    catalog: Catalog,
    packages: PackageCatalog,
}

impl Session {
    async fn start() -> Self {
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
        Session {
            ctx,
            catalog,
            packages: PackageCatalog::load(),
        }
    }

    /// Add a code cell holding `input`, run it, and return its output.
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
        let result = evaluate_cell(
            &self.ctx,
            &cell_id,
            &self.catalog,
            &self.packages,
            30,
            &NoEffects,
        )
        .await
        .expect("the cell runs");
        assert!(
            !result.cell_output.is_error,
            "{input}: {:?}",
            result.cell_output
        );
        result.cell_output
    }

    async fn latex(&self, input: &str) -> String {
        let output = self.run(input).await;
        match output.latex {
            Some(latex) => latex,
            None => panic!("{input}: no LaTeX in {:?}", output.text_output),
        }
    }
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn letter_and_ascii_name_are_one_variable() {
    let s = Session::start().await;
    s.run("α: 2$").await;
    assert_eq!(s.latex("alpha + 1;").await, "3");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn gamma_before_a_bracket_is_the_gamma_function() {
    let s = Session::start().await;
    assert_eq!(s.latex("Γ(5);").await, "24");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn two_forms_of_phi_are_one_variable() {
    let s = Session::start().await;
    s.run("ϕ: 2$").await;
    assert_eq!(s.latex("φ + 1;").await, "3");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn combining_accent_names_the_same_variable() {
    let s = Session::start().await;
    s.run("\u{00E9}: 1$").await;
    assert_eq!(s.latex("e\u{0301} + 1;").await, "2");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn epsilon_phi_and_theta_display_as_typed() {
    let s = Session::start().await;
    let latex = s.latex("[ε, φ, θ];").await;
    for name in ["\\varepsilon", "\\varphi", "\\theta"] {
        assert!(latex.contains(name), "{latex} lacks {name}");
    }
    assert!(!latex.contains("\\vartheta"), "{latex}");
}

#[tokio::test]
#[ignore = "requires Maxima to be installed"]
async fn greek_inside_a_name_displays_as_typed() {
    let s = Session::start().await;
    let latex = s.latex("Δx;").await;
    assert!(latex.contains("Δx"), "{latex}");
}
