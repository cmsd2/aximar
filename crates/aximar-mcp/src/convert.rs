use aximar_core::notebook::CellOutput;
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
                let mut entries = Vec::new();
                // Intermediate/print text as a stream output
                if !o.text_output.is_empty() {
                    entries.push(serde_json::json!({
                        "output_type": "stream",
                        "name": "stdout",
                        "text": [&o.text_output],
                    }));
                }
                // Final result as execute_result with text/latex
                if let Some(ref latex) = o.latex {
                    entries.push(serde_json::json!({
                        "output_type": "execute_result",
                        "data": { "text/latex": [latex] },
                        "metadata": {},
                        "execution_count": execution_count,
                    }));
                }
                // Plot data as display_data with custom MIME types
                if let Some(ref plot_data) = o.plot_data {
                    entries.push(serde_json::json!({
                        "output_type": "display_data",
                        "data": { "application/x-maxima-plotly": [plot_data] },
                        "metadata": {},
                    }));
                }
                if let Some(ref plot_svg) = o.plot_svg {
                    entries.push(serde_json::json!({
                        "output_type": "display_data",
                        "data": { "image/svg+xml": [plot_svg] },
                        "metadata": {},
                    }));
                }
                entries
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

/// Convert an ipynb Notebook into a list of (id, cell_type, input, output) tuples
/// suitable for the LoadCells command.
pub(crate) fn ipynb_to_cell_tuples(
    notebook: &notebook_types::Notebook,
) -> Vec<(String, CellType, String, Option<CellOutput>)> {
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
            let output = parse_nbformat_outputs(cell);
            Some((id, cell_type, input, output))
        })
        .collect()
}

/// Parse nbformat cell outputs into a CellOutput.
///
/// In nbformat, each output entry's `data` dict holds alternative MIME
/// representations of the *same* value — text/plain is a fallback for
/// text/latex, not additional content. So within a single execute_result or
/// display_data entry, we prefer text/latex and only fall back to text/plain
/// when no text/latex exists. Stream outputs are genuinely separate content
/// (print output, intermediate results) and always go to text_output.
fn parse_nbformat_outputs(cell: &notebook_types::NotebookCell) -> Option<CellOutput> {
    let outputs = cell.outputs.as_ref()?;
    if outputs.is_empty() {
        return None;
    }

    let mut text_output = String::new();
    let mut latex: Option<String> = None;
    let mut plot_data: Option<String> = None;
    let mut plot_svg: Option<String> = None;
    let mut execution_count = cell.execution_count.map(|c| c as u32);

    for raw in outputs {
        let output_type = raw.get("output_type").and_then(|v| v.as_str()).unwrap_or("");
        match output_type {
            "execute_result" | "display_data" => {
                if let Some(data) = raw.get("data") {
                    if let Some(plotly) = data.get("application/x-maxima-plotly") {
                        plot_data = Some(join_string_or_array(plotly));
                    }
                    if let Some(svg) = data.get("image/svg+xml") {
                        plot_svg = Some(join_string_or_array(svg));
                    }
                    if let Some(tex) = data.get("text/latex") {
                        // Prefer LaTeX; text/plain is just a fallback for the same value
                        latex = Some(join_string_or_array(tex));
                    } else if let Some(plain) = data.get("text/plain") {
                        // No LaTeX — use text/plain as text output
                        text_output.push_str(&join_string_or_array(plain));
                    }
                }
                if output_type == "execute_result" {
                    if let Some(ec) = raw.get("execution_count").and_then(|v| v.as_u64()) {
                        execution_count = Some(ec as u32);
                    }
                }
            }
            "stream" => {
                if let Some(text) = raw.get("text") {
                    text_output.push_str(&join_string_or_array(text));
                }
            }
            _ => {}
        }
    }

    if text_output.is_empty() && latex.is_none() && plot_data.is_none() && plot_svg.is_none() {
        return None;
    }

    Some(CellOutput {
        text_output,
        latex,
        plot_svg,
        plot_data,
        error: None,
        is_error: false,
        duration_ms: 0,
        output_label: None,
        execution_count,
    })
}

/// Join a JSON value that is either a string or an array of strings.
fn join_string_or_array(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}
