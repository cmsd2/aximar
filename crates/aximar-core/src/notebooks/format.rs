//! The `.macnb` notebook format for Aximar's own notebook model.  The
//! format itself (reading, writing, cell IDs, the output mapping) lives in
//! the `aximar-notebook-format` crate, which the VS Code extension also uses
//! as WebAssembly; this module converts between that crate's file model and
//! `Notebook`, and works out friendly explanations for loaded errors.
//! `docs/notebook-format.md` describes the format.

use serde_json::{Map, Value};

use aximar_notebook_format as file;
pub use aximar_notebook_format::FormatError;

use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::maxima::errors::enhance_error_with_packages;
use crate::notebook::{CellOutput, CellType, Notebook};

/// A notebook read from a file, ready for `NotebookCommand::LoadNotebook`.
#[derive(Debug, Clone)]
pub struct NotebookFile {
    pub metadata: Map<String, Value>,
    pub cells: Vec<LoadedCell>,
    /// Some cells had no usable ID and were given new ones, so the file
    /// should be saved again.
    pub ids_replaced: bool,
}

/// One cell read from a file.
#[derive(Debug, Clone)]
pub struct LoadedCell {
    pub id: String,
    pub cell_type: CellType,
    pub input: String,
    pub metadata: Map<String, Value>,
    /// Fields of the cell Aximar doesn't use, written back unchanged.
    pub extra: Map<String, Value>,
    pub output: Option<CellOutput>,
}

// ── Reading ──────────────────────────────────────────────────────────

/// Read a notebook file.  Outputs come back stale: they belong to a
/// session that no longer exists.
pub fn read(json: &str) -> Result<NotebookFile, FormatError> {
    let f = file::read(json)?;
    let cells = f
        .cells
        .into_iter()
        .map(|c| LoadedCell {
            id: c.id,
            cell_type: cell_type_from(c.cell_type),
            input: c.input,
            metadata: c.metadata,
            extra: c.extra,
            output: c.output.map(output_from),
        })
        .collect();
    Ok(NotebookFile {
        metadata: f.metadata,
        cells,
        ids_replaced: f.ids_replaced,
    })
}

fn cell_type_from(t: file::CellType) -> CellType {
    match t {
        file::CellType::Code => CellType::Code,
        file::CellType::Markdown => CellType::Markdown,
        file::CellType::Raw => CellType::Raw,
    }
}

fn cell_type_to(t: CellType) -> file::CellType {
    match t {
        CellType::Code => file::CellType::Code,
        CellType::Markdown => file::CellType::Markdown,
        CellType::Raw => file::CellType::Raw,
    }
}

fn output_from(o: file::Output) -> CellOutput {
    CellOutput {
        text_output: o.text_output,
        latex: o.latex,
        text_plain: o.text_plain,
        plot_svg: o.plot_svg,
        plot_data: o.plot_data,
        image_png: o.image_png,
        error: o.error,
        is_error: o.is_error,
        execution_count: o.execution_count,
        stale: o.stale,
        other_outputs: o.other_outputs,
        ..Default::default()
    }
}

fn output_to(o: &CellOutput) -> file::Output {
    file::Output {
        text_output: o.text_output.clone(),
        latex: o.latex.clone(),
        text_plain: o.text_plain.clone(),
        plot_svg: o.plot_svg.clone(),
        plot_data: o.plot_data.clone(),
        image_png: o.image_png.clone(),
        error: o.error.clone(),
        is_error: o.is_error,
        execution_count: o.execution_count,
        stale: o.stale,
        other_outputs: o.other_outputs.clone(),
    }
}

/// Work out the friendly explanation for each error read from a file.
/// Files store only the error text; the explanation depends on the catalog
/// and the installed packages, so it is computed again when a file opens.
pub fn explain_errors(file: &mut NotebookFile, catalog: &Catalog, packages: &PackageCatalog) {
    for output in file.cells.iter_mut().filter_map(|c| c.output.as_mut()) {
        if let Some(error) = output.error.as_deref().filter(|_| output.is_error) {
            output.error_info = enhance_error_with_packages(error, catalog, Some(packages));
        }
    }
}

// ── Writing ──────────────────────────────────────────────────────────

/// Write a notebook as nbformat 4.5 JSON.
pub fn write(notebook: &Notebook) -> String {
    file::write(&file::NotebookFile {
        metadata: notebook.metadata().clone(),
        cells: notebook
            .cells()
            .iter()
            .map(|cell| file::Cell {
                id: cell.id.clone(),
                cell_type: cell_type_to(cell.cell_type),
                input: cell.input.clone(),
                metadata: cell.metadata.clone(),
                extra: cell.extra.clone(),
                output: cell.output.as_ref().map(output_to),
            })
            .collect(),
        ids_replaced: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::NotebookCommand;
    use aximar_notebook_format::text;
    use serde_json::json;

    fn fixture(name: &str) -> String {
        let path = format!(
            "{}/../../packages/aximar-notebook-format/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    /// Read a fixture and load it into a notebook.
    fn load(name: &str) -> (NotebookFile, Notebook) {
        let file = read(&fixture(name)).unwrap();
        let mut nb = Notebook::new();
        nb.apply(NotebookCommand::LoadNotebook {
            file: file.clone().into(),
        })
        .unwrap();
        (file, nb)
    }

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn cell<'a>(nb: &'a Notebook, input: &str) -> &'a crate::notebook::Cell {
        nb.cells()
            .iter()
            .find(|c| c.input == input)
            .unwrap_or_else(|| panic!("no cell {input:?}"))
    }

    #[test]
    fn the_spec_example_round_trips_unchanged() {
        let (file, nb) = load("spec-example.macnb");
        assert!(!file.ids_replaced);
        assert_eq!(json(&write(&nb)), json(&fixture("spec-example.macnb")));
    }

    #[test]
    fn the_vscode_file_reads_plots_as_plots_and_keeps_errors() {
        let (_, nb) = load("vscode-extension-before-s1.macnb");
        // The text/plain copy of the SVG source is not shown (#6)
        let plot = cell(&nb, "plot2d(x^2, [x, 0, 1])$")
            .output
            .as_ref()
            .unwrap();
        assert!(plot.plot_svg.is_some());
        assert!(plot.text_output.is_empty(), "{:?}", plot.text_output);
        let plotly = cell(&nb, "plot2d(x, [x, 0, 1], [plot_format, plotly])$")
            .output
            .as_ref()
            .unwrap();
        assert!(plotly.plot_data.is_some() && plotly.text_output.is_empty());
        // Print text written as a text/plain bundle is print text
        let result = cell(&nb, "print(\"working\")$\nintegrate(x, x);");
        let out = result.output.as_ref().unwrap();
        assert_eq!(out.text_output, "working");
        assert_eq!(out.latex.as_deref(), Some("{{x^2}\\over{2}}"));
        // The extension's own metadata is kept
        assert_eq!(result.metadata["outputLabel"], "%o8");
        // Its error output is an error (#7)
        let failed = cell(&nb, "1/0;");
        let out = failed.output.as_ref().unwrap();
        assert!(out.is_error);
        assert!(out.error.as_deref().unwrap().starts_with("expt: undefined"));
        assert_eq!(failed.status, crate::notebook::CellStatus::Error);
    }

    #[test]
    fn the_jupyter_file_keeps_raw_cells_metadata_and_plain_results() {
        let (file, nb) = load("jupyter-4.0.ipynb");
        assert!(file.ids_replaced);
        assert_eq!(nb.cells()[0].cell_type, CellType::Raw);
        assert_eq!(nb.metadata()["jupytext"]["formats"], "ipynb,md");
        let code = cell(&nb, "x: 2$\nx^2;");
        assert_eq!(code.metadata["tags"], json(r#"["setup"]"#));
        let out = code.output.as_ref().unwrap();
        assert_eq!(out.text_plain.as_deref(), Some("4"));
        assert_eq!(out.execution_count, Some(5));
        // Written back, everything but the new IDs and the version is the same
        let mut original = json(&fixture("jupyter-4.0.ipynb"));
        let mut written = json(&write(&nb));
        for v in [&mut original, &mut written] {
            v["nbformat_minor"] = json!(0);
            for c in v["cells"].as_array_mut().unwrap() {
                c.as_object_mut().unwrap().remove("id");
                // Sources are written as lines
                c["source"] = json!(text(Some(&c["source"])));
            }
        }
        assert_eq!(written, original);
    }

    #[test]
    fn files_saved_before_s1_open() {
        let (_, nb) = load("aximar-mcp-before-s1.macnb");
        assert!(cell(&nb, "plot2d(x^2, [x, 0, 1])$")
            .output
            .as_ref()
            .unwrap()
            .plot_svg
            .is_some());
        // The failed cell was saved with no outputs (#7)
        assert!(cell(&nb, "1/0;").output.is_none());
        let (_, nb) = load("aximar-window-before-s1.macnb");
        assert_eq!(
            cell(&nb, "print(\"working\")$\nintegrate(x, x);")
                .output
                .as_ref()
                .unwrap()
                .text_output,
            "working"
        );
    }

    #[test]
    fn an_error_in_the_print_text_is_written_once() {
        let mut nb = Notebook::new();
        let id = nb.cells()[0].id.clone();
        let output = CellOutput {
            text_output: "incorrect syntax: x is not an infix operator\n".into(),
            is_error: true,
            execution_count: Some(1),
            ..Default::default()
        };
        nb.apply(NotebookCommand::SetCellOutput {
            cell_id: id,
            output: Box::new(output),
            raw_output: Vec::new(),
        })
        .unwrap();
        let v = json(&write(&nb));
        let outputs = v["cells"][0]["outputs"].as_array().unwrap();
        assert_eq!(outputs.len(), 1, "{outputs:?}");
        assert_eq!(outputs[0]["output_type"], "error");
        assert_eq!(
            outputs[0]["evalue"],
            "incorrect syntax: x is not an infix operator"
        );
    }

    #[test]
    fn unknown_outputs_are_kept() {
        let text = r#"{"nbformat": 4, "nbformat_minor": 5, "metadata": {}, "cells": [
            {"id": "c", "cell_type": "code", "source": "1;", "metadata": {}, "execution_count": 1, "outputs": [
                {"output_type": "display_data", "data": {"text/html": ["<b>x</b>"]}, "metadata": {}},
                {"output_type": "future_type", "payload": 1}
            ]}]}"#;
        let file = read(text).unwrap();
        let mut nb = Notebook::new();
        nb.apply(NotebookCommand::LoadNotebook { file: file.into() })
            .unwrap();
        let v = json(&write(&nb));
        let outputs = v["cells"][0]["outputs"].as_array().unwrap();
        assert_eq!(outputs[0]["data"]["text/html"], json(r#"["<b>x</b>"]"#));
        assert_eq!(outputs[1]["output_type"], "future_type");
    }

    #[test]
    fn other_cell_fields_are_kept() {
        let text = r#"{"nbformat": 4, "nbformat_minor": 5, "metadata": {}, "cells": [
            {"id": "m", "cell_type": "markdown", "source": "![x](attachment:x.png)", "metadata": {},
             "attachments": {"x.png": {"image/png": "iVBORw0KGgo="}}}]}"#;
        let file = read(text).unwrap();
        let mut nb = Notebook::new();
        nb.apply(NotebookCommand::LoadNotebook { file: file.into() })
            .unwrap();
        let v = json(&write(&nb));
        assert_eq!(
            v["cells"][0]["attachments"]["x.png"]["image/png"],
            "iVBORw0KGgo="
        );
    }

    #[test]
    fn loaded_errors_get_their_friendly_explanation() {
        let mut file = read(&fixture("vscode-extension-before-s1.macnb")).unwrap();
        explain_errors(&mut file, &Catalog::load(), &PackageCatalog::load());
        let failed = file.cells.iter().find(|c| c.input == "1/0;").unwrap();
        let info = failed
            .output
            .as_ref()
            .unwrap()
            .error_info
            .as_ref()
            .expect("an explanation");
        assert_eq!(info.title, "Division by Zero");
        // Outputs that aren't errors get none
        assert!(file
            .cells
            .iter()
            .filter(|c| c.input != "1/0;")
            .all(|c| c.output.as_ref().is_none_or(|o| o.error_info.is_none())));
    }
}
