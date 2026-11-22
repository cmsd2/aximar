use std::sync::Arc;
use tokio::sync::Mutex;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::{NoEffects, NotebookCommand};
use aximar_core::evaluation::{run_all, CellRun, CellRunStatus, RunAllOptions};
use aximar_core::notebooks::io as notebook_io;
use aximar_core::notebook::CellType;
use aximar_core::registry::NotebookRegistry;
use aximar_core::safety::TrustMode;

use crate::convert::{ipynb_to_cell_tuples, notebook_to_ipynb};
use crate::server::ServerCore;

/// Run a notebook from the command line: load, execute all cells, save with outputs.
///
/// Running `aximar-mcp run` on a notebook is the decision to run it, as with
/// `maxima -b` or `jupyter nbconvert --execute`, so every cell runs: the
/// dangerous-call check does not apply.  The check matches names in the cell
/// text, so it could not stop a notebook written to get past it anyway.
pub async fn run(input_path: &str, output_path: Option<&str>) -> anyhow::Result<()> {
    let output_path = output_path.unwrap_or(input_path);

    // Load catalog and packages
    let catalog = Arc::new(Catalog::load());
    let packages = Arc::new(PackageCatalog::load());

    // Read configuration from environment
    let session_settings = crate::config::session_settings_from_env();
    let eval_timeout = session_settings.eval_timeout;

    // Read notebook
    let notebook = notebook_io::read_notebook(input_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let cell_tuples = ipynb_to_cell_tuples(&notebook);
    let total_code_cells = cell_tuples
        .iter()
        .filter(|(_, ct, _, _)| *ct == CellType::Code)
        .count();

    eprintln!("Loaded {} cells ({} code) from {}", cell_tuples.len(), total_code_cells, input_path);

    // Create registry with one notebook, set path so Maxima CWD is the notebook dir
    let notebook_path = std::path::Path::new(input_path).canonicalize()?;
    let registry = Arc::new(Mutex::new(NotebookRegistry::new()));
    let ctx = {
        let mut reg = registry.lock().await;
        let id = reg.active_id().to_string();
        reg.set_path(&id, Some(notebook_path)).map_err(|e| anyhow::anyhow!("{e}"))?;
        reg.resolve(None).map_err(|e| anyhow::anyhow!("{e}"))?
    };
    {
        let mut nb = ctx.notebook.lock().await;
        nb.apply(NotebookCommand::LoadCells { cells: cell_tuples })
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Build server core and start Maxima
    let core = ServerCore::new(
        registry,
        catalog.clone(),
        packages.clone(),
        session_settings.fixed(),
        true,
    );
    core.ensure_session(&ctx)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start Maxima: {e}"))?;

    // Run every code cell, printing each as it finishes
    let opts = RunAllOptions {
        catalog: &catalog,
        packages: &packages,
        eval_timeout,
        mode: TrustMode::Headless { allow_dangerous: true },
        effects: &NoEffects,
    };
    let mut done = 0;
    let report = run_all(&ctx, &opts, &mut |run: &CellRun| {
        done += 1;
        let progress = format!("[{done}/{total_code_cells}]");
        let seconds = run.duration_ms.unwrap_or(0) as f64 / 1000.0;
        match run.status {
            CellRunStatus::Ok => eprintln!("{progress} ok ({seconds:.1}s)"),
            CellRunStatus::Empty => eprintln!("{progress} skipped (empty)"),
            CellRunStatus::Error => {
                eprintln!("{progress} ERROR ({seconds:.1}s)");
                if let Some(ref err) = run.error {
                    eprintln!("  Error: {err}");
                }
            }
            CellRunStatus::Failed => {
                eprintln!("{progress} FAILED: {}", run.error.as_deref().unwrap_or(""));
            }
            CellRunStatus::NeedsTrust | CellRunStatus::NotRun => {}
        }
    })
    .await;

    if report.stop_reason.is_some() {
        // The cell's error is printed above; leave the notebook unsaved
        std::process::exit(1);
    }

    // Convert back and save
    let ipynb = {
        let nb = ctx.notebook.lock().await;
        notebook_to_ipynb(&nb)
    };
    notebook_io::write_notebook(output_path, &ipynb)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    eprintln!("Saved to {}", output_path);
    Ok(())
}
