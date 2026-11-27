use std::sync::Arc;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::{NoEffects, NotebookCommand};
use aximar_core::error::AppError;
use aximar_core::evaluation::evaluate_cell;
use aximar_core::maxima::backend::Backend;
use aximar_core::maxima::output::OutputSink;
use aximar_core::notebook::{CellOutput, CellType};
use aximar_core::registry::{NotebookContextRef, NotebookRegistry};
use aximar_core::session_ops::{ensure_session, SessionSettings};

pub struct Session {
    pub ctx: NotebookContextRef,
    catalog: Catalog,
    packages: PackageCatalog,
}

impl Session {
    pub async fn start() -> Self {
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

    /// Add a code cell holding `input`, run it with `timeout` seconds, and
    /// return the run's result and the cell's output afterwards
    pub async fn run(
        &self,
        input: &str,
        timeout: u64,
    ) -> (Result<(), AppError>, Option<CellOutput>) {
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
            timeout,
            &NoEffects,
        )
        .await
        .map(|_| ());
        let nb = self.ctx.notebook.lock().await;
        let output = nb.get_cell(&cell_id).unwrap().output.clone();
        (result, output)
    }
}
