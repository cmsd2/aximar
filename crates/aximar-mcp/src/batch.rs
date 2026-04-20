use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::commands::NotebookCommand;
use aximar_core::error::AppError;
use aximar_core::evaluation::evaluate_cell;
use aximar_core::notebooks::io as notebook_io;
use aximar_core::notebook::CellType;
use aximar_core::registry::NotebookRegistry;
use aximar_core::safety;

use crate::convert::{ipynb_to_cell_tuples, notebook_to_ipynb};
use crate::server::ServerCore;

/// Run a notebook from the command line: load, execute all cells, save with outputs.
pub async fn run(args: Vec<String>) -> anyhow::Result<()> {
    // Parse args: run <path> [-o <output>] [--allow-dangerous]
    let mut input_path: Option<String> = None;
    let mut output_path: Option<String> = None;
    let mut allow_dangerous = false;

    let mut i = 2; // skip binary name and "run"
    while i < args.len() {
        match args[i].as_str() {
            "-o" => {
                i += 1;
                output_path = args.get(i).cloned();
            }
            "--allow-dangerous" => {
                allow_dangerous = true;
            }
            arg if !arg.starts_with('-') && input_path.is_none() => {
                input_path = Some(arg.to_string());
            }
            other => {
                anyhow::bail!("Unknown argument: {other}");
            }
        }
        i += 1;
    }

    let input_path = input_path.ok_or_else(|| {
        anyhow::anyhow!("Usage: aximar-mcp run <path> [-o <output>] [--allow-dangerous]")
    })?;
    let output_path = output_path.unwrap_or_else(|| input_path.clone());

    if allow_dangerous {
        eprintln!("Warning: --allow-dangerous enabled");
    }

    // Load catalog and packages
    let catalog = Arc::new(Catalog::load());
    let packages = Arc::new(PackageCatalog::load());

    // Read configuration from environment
    let backend = crate::config::backend_from_env();
    let maxima_path = crate::config::maxima_path_from_env();
    let eval_timeout = crate::config::eval_timeout_from_env();

    // Read notebook
    let notebook = notebook_io::read_notebook(&input_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let cell_tuples = ipynb_to_cell_tuples(&notebook);
    let total_code_cells = cell_tuples
        .iter()
        .filter(|(_, ct, _, _)| *ct == CellType::Code)
        .count();

    eprintln!("Loaded {} cells ({} code) from {}", cell_tuples.len(), total_code_cells, input_path);

    // Create registry with one notebook, load cells
    let registry = Arc::new(Mutex::new(NotebookRegistry::new()));
    let ctx = {
        let reg = registry.lock().await;
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
        backend,
        maxima_path,
        eval_timeout,
        allow_dangerous,
    );
    core.ensure_session(&ctx)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start Maxima: {e}"))?;

    // Collect code cell IDs
    let cell_ids: Vec<String> = {
        let nb = ctx.notebook.lock().await;
        nb.cells()
            .iter()
            .filter(|c| c.cell_type == CellType::Code)
            .map(|c| c.id.clone())
            .collect()
    };

    // Execute cells
    let mut code_idx = 0;
    for cell_id in &cell_ids {
        code_idx += 1;

        // Safety check
        if !allow_dangerous {
            let input = {
                let nb = ctx.notebook.lock().await;
                nb.get_cell(cell_id).map(|c| c.input.clone()).unwrap_or_default()
            };
            let dangerous = safety::detect_dangerous_calls(&input, Some(&packages));
            if !dangerous.is_empty() {
                let names: Vec<&str> = dangerous.iter().map(|d| d.function_name.as_str()).collect();
                anyhow::bail!(
                    "Cell {} contains dangerous function(s): {}. Use --allow-dangerous to allow.",
                    cell_id,
                    names.join(", ")
                );
            }
        }

        let start = Instant::now();
        match evaluate_cell(&ctx, cell_id, &catalog, &packages, eval_timeout).await {
            Ok(result) => {
                let elapsed = start.elapsed();
                let status = if result.cell_output.is_error {
                    "ERROR"
                } else {
                    "ok"
                };
                eprintln!(
                    "[{}/{}] {} ({:.1}s)",
                    code_idx,
                    total_code_cells,
                    status,
                    elapsed.as_secs_f64()
                );
                if result.cell_output.is_error {
                    if let Some(ref err) = result.cell_output.error {
                        eprintln!("  Error: {}", err);
                    }
                    std::process::exit(1);
                }
            }
            Err(AppError::EmptyInput) => {
                eprintln!("[{}/{}] skipped (empty)", code_idx, total_code_cells);
            }
            Err(AppError::CellIsMarkdown) => {
                eprintln!("[{}/{}] skipped (markdown)", code_idx, total_code_cells);
            }
            Err(e) => {
                eprintln!("[{}/{}] FAILED: {}", code_idx, total_code_cells, e);
                std::process::exit(1);
            }
        }
    }

    // Convert back and save
    let ipynb = {
        let nb = ctx.notebook.lock().await;
        notebook_to_ipynb(&nb)
    };
    notebook_io::write_notebook(&output_path, &ipynb)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    eprintln!("Saved to {}", output_path);
    Ok(())
}
