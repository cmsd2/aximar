//! Cells waiting in a run of all cells show as queued, and get their old
//! status back when the run stops before them.  No Maxima needed: the trust
//! check stops the run at its first cell.

use std::sync::Mutex;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::{CommandEffect, EffectSink, NotebookCommand};
use aximar_core::evaluation::{run_all, RunAllOptions, StopReason};
use aximar_core::notebook::{CellStatus, CellType, Notebook};
use aximar_core::registry::NotebookRegistry;
use aximar_core::safety::TrustMode;

/// Records every cell's status each time a change is reported
#[derive(Default)]
struct Statuses(Mutex<Vec<Vec<CellStatus>>>);

impl EffectSink for Statuses {
    fn effect(&self, _notebook_id: &str, nb: &Notebook, _effect: &CommandEffect) {
        self.0
            .lock()
            .unwrap()
            .push(nb.cells().iter().map(|c| c.status).collect());
    }
}

#[tokio::test]
async fn cells_wait_as_queued_and_get_their_status_back() {
    let mut registry = NotebookRegistry::new();
    let id = registry.create_empty();
    let ctx = registry.get(&id).unwrap();
    {
        let mut nb = ctx.notebook.lock().await;
        let mut ids = Vec::new();
        for input in ["system(\"ls\")$", "y: 2;", ""] {
            let effect = nb
                .apply(NotebookCommand::AddCell {
                    cell_type: CellType::Code,
                    input: input.into(),
                    after_cell_id: ids.last().cloned(),
                    before_cell_id: None,
                })
                .unwrap();
            ids.push(effect.cell_id().unwrap().to_string());
        }
        // The second cell ran before
        nb.apply(NotebookCommand::SetCellStatus {
            cell_id: ids[1].clone(),
            status: CellStatus::Success,
        })
        .unwrap();
    }

    let (catalog, packages) = (Catalog::load(), PackageCatalog::load());
    let sink = Statuses::default();
    let opts = RunAllOptions {
        catalog: &catalog,
        packages: &packages,
        eval_timeout: 30,
        // No one to ask, and dangerous calls not allowed: system() stops the run
        mode: TrustMode::Headless {
            allow_dangerous: false,
        },
        effects: &sink,
    };
    let report = run_all(&ctx, &opts, &mut |_| {}).await;
    assert_eq!(report.stop_reason, Some(StopReason::NeedsTrust));

    let seen = sink.0.lock().unwrap().clone();
    // First, every cell with code waits; the blank one doesn't
    assert_eq!(
        seen.first().unwrap(),
        &vec![CellStatus::Queued, CellStatus::Queued, CellStatus::Idle]
    );
    // At the end, each cell has its status from before the run
    let nb = ctx.notebook.lock().await;
    assert_eq!(
        nb.cells().iter().map(|c| c.status).collect::<Vec<_>>(),
        vec![CellStatus::Idle, CellStatus::Success, CellStatus::Idle]
    );
}
