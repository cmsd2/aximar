use aximar_core::notebooks::types as notebook_types;

use crate::notebook::{CellType, Notebook};

/// Convert internal Notebook to Jupyter ipynb format for saving.
pub(crate) fn notebook_to_ipynb(nb: &Notebook) -> notebook_types::Notebook {
    let cells: Vec<notebook_types::NotebookCell> = nb
        .cells()
        .iter()
        .map(|cell| {
            let cell_type = match cell.cell_type {
                CellType::Code => notebook_types::CellType::Code,
                CellType::Markdown => notebook_types::CellType::Markdown,
            };
            let execution_count = cell
                .output
                .as_ref()
                .and_then(|o| o.execution_count)
                .map(|c| c as u64);

            let outputs = cell.output.as_ref().map(|o| {
                let mut out = serde_json::json!({
                    "output_type": "execute_result",
                    "text/plain": o.text_output,
                });
                if let Some(ref latex) = o.latex {
                    out["text/latex"] = serde_json::json!(latex);
                }
                vec![out]
            });

            notebook_types::NotebookCell {
                cell_type,
                source: notebook_types::CellSource::String(cell.input.clone()),
                metadata: serde_json::json!({}),
                execution_count,
                outputs,
            }
        })
        .collect();

    notebook_types::Notebook {
        nbformat: 4,
        nbformat_minor: 5,
        metadata: notebook_types::NotebookMetadata {
            kernelspec: notebook_types::KernelSpec {
                name: "maxima".into(),
                display_name: "Maxima".into(),
                language: Some("maxima".into()),
            },
            aximar: Some(notebook_types::AximarMetadata {
                template_id: None,
                title: None,
                description: None,
            }),
        },
        cells,
    }
}

/// Convert an ipynb Notebook into a list of (id, cell_type, input) tuples
/// suitable for the LoadCells command.
pub(crate) fn ipynb_to_cell_tuples(
    notebook: &notebook_types::Notebook,
) -> Vec<(String, CellType, String)> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LOAD_COUNTER: AtomicU64 = AtomicU64::new(1);

    notebook
        .cells
        .iter()
        .filter_map(|cell| {
            let cell_type = match cell.cell_type {
                notebook_types::CellType::Code => CellType::Code,
                notebook_types::CellType::Markdown => CellType::Markdown,
                notebook_types::CellType::Raw => return None,
            };
            let input = match &cell.source {
                notebook_types::CellSource::String(s) => s.clone(),
                notebook_types::CellSource::Lines(lines) => lines.join(""),
            };
            let id = format!("load-{}", LOAD_COUNTER.fetch_add(1, Ordering::Relaxed));
            Some((id, cell_type, input))
        })
        .collect()
}
