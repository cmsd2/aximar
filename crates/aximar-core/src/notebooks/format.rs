//! The `.macnb` notebook format: one reader and writer for the desktop app,
//! MCP and batch runs.  `docs/notebook-format.md` describes the format;
//! the `notebook-format` spec states the rules this follows.

use serde_json::{Map, Value};

use crate::notebook::{CellOutput, CellType};

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

use std::collections::HashSet;

use serde_json::json;

use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::maxima::errors::enhance_error_with_packages;
use crate::notebook::{new_cell_id, Notebook};

/// Why a file couldn't be read as a notebook.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not a notebook: {0}")]
    Shape(String),
    #[error("nbformat {0} isn't supported; Aximar reads nbformat 4")]
    Version(u64),
}

const PLOTLY: &str = "application/x-maxima-plotly";
const SVG: &str = "image/svg+xml";
const PNG: &str = "image/png";
const LATEX: &str = "text/latex";
const PLAIN: &str = "text/plain";

/// Fields of a cell this module reads; anything else is kept as is.
const CELL_FIELDS: &[&str] = &["id", "cell_type", "source", "metadata", "execution_count", "outputs"];

// ── Reading ──────────────────────────────────────────────────────────

/// Read a notebook file.  Outputs come back stale: they belong to a
/// session that no longer exists.
pub fn read(json: &str) -> Result<NotebookFile, FormatError> {
    let root: Value = serde_json::from_str(json)?;
    let root = root
        .as_object()
        .ok_or_else(|| FormatError::Shape("the top level isn't an object".into()))?;
    match root.get("nbformat").and_then(Value::as_u64) {
        Some(4) => {}
        Some(other) => return Err(FormatError::Version(other)),
        None => return Err(FormatError::Shape("no nbformat version".into())),
    }
    let metadata = root.get("metadata").and_then(Value::as_object).cloned().unwrap_or_default();

    let mut seen = HashSet::new();
    let mut ids_replaced = false;
    let mut cells = Vec::new();
    for raw in root.get("cells").and_then(Value::as_array).into_iter().flatten() {
        let Some(cell) = raw.as_object() else {
            return Err(FormatError::Shape("a cell isn't an object".into()));
        };
        let id = match cell.get("id").and_then(Value::as_str) {
            Some(id) if valid_id(id) && seen.insert(id.to_string()) => id.to_string(),
            _ => {
                ids_replaced = true;
                let id = new_cell_id();
                seen.insert(id.clone());
                id
            }
        };
        let cell_type = match cell.get("cell_type").and_then(Value::as_str) {
            Some("code") => CellType::Code,
            Some("markdown") => CellType::Markdown,
            // Raw cells, and any type a later nbformat adds, are kept as raw
            _ => CellType::Raw,
        };
        let output = match cell_type {
            CellType::Code => read_outputs(
                cell.get("outputs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default(),
                cell.get("execution_count").and_then(Value::as_u64).map(|n| n as u32),
            ),
            _ => None,
        };
        let extra = cell
            .iter()
            .filter(|(key, _)| !CELL_FIELDS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        cells.push(LoadedCell {
            id,
            cell_type,
            input: text(cell.get("source")),
            metadata: cell.get("metadata").and_then(Value::as_object).cloned().unwrap_or_default(),
            extra,
            output,
        });
    }
    Ok(NotebookFile { metadata, cells, ids_replaced })
}

/// nbformat 4.5's rule: 1 to 64 letters, digits, `-` or `_`.
fn valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A string, or a list of strings joined (nbformat's multiline text).
fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts.iter().filter_map(Value::as_str).collect(),
        _ => String::new(),
    }
}

/// Fill `slot` with `value`, or keep `output` for writing back if a value
/// of that kind came first: one cell shows one of each.
fn take(slot: &mut Option<String>, value: String, output: &Value, others: &mut Vec<Value>) {
    if slot.is_none() {
        *slot = Some(value);
    } else {
        others.push(output.clone());
    }
}

fn read_outputs(outputs: &[Value], execution_count: Option<u32>) -> Option<CellOutput> {
    if outputs.is_empty() {
        return None;
    }
    let mut out = CellOutput { execution_count, stale: true, ..Default::default() };
    let mut others = Vec::new();
    for output in outputs {
        match output.get("output_type").and_then(Value::as_str) {
            Some("stream") => out.text_output.push_str(&text(output.get("text"))),
            Some("error") => {
                out.is_error = true;
                let message = text(output.get("evalue"));
                out.error = Some(if message.is_empty() { text(output.get("ename")) } else { message });
            }
            Some(kind @ ("execute_result" | "display_data")) => {
                let data = output.get("data").and_then(Value::as_object);
                let get = |mime: &str| data.and_then(|d| d.get(mime)).map(|v| text(Some(v)));
                // One value per bundle, the richest; text/plain only when
                // nothing richer is there
                if let Some(v) = get(PLOTLY) {
                    take(&mut out.plot_data, v, output, &mut others);
                } else if let Some(v) = get(SVG) {
                    take(&mut out.plot_svg, v, output, &mut others);
                } else if let Some(v) = get(PNG) {
                    take(&mut out.image_png, v, output, &mut others);
                } else if let Some(v) = get(LATEX) {
                    if out.latex.is_some() {
                        others.push(output.clone());
                    } else {
                        out.latex = Some(v);
                        out.text_plain = get(PLAIN);
                    }
                } else if let Some(v) = get(PLAIN) {
                    let counted = output.get("execution_count").is_some_and(|n| !n.is_null());
                    if kind == "execute_result" && counted && out.text_plain.is_none() {
                        // A result with only a plain form, as Jupyter writes it
                        out.text_plain = Some(v);
                    } else {
                        // Print text that a writer put in a bundle
                        out.text_output.push_str(&v);
                    }
                } else {
                    others.push(output.clone());
                }
                if kind == "execute_result" {
                    if let Some(n) = output.get("execution_count").and_then(Value::as_u64) {
                        out.execution_count = Some(n as u32);
                    }
                }
            }
            _ => others.push(output.clone()),
        }
    }
    out.other_outputs = others;
    Some(out)
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
    let mut metadata = notebook.metadata().clone();
    metadata.insert(
        "kernelspec".into(),
        json!({ "name": "maxima", "display_name": "Maxima", "language": "maxima" }),
    );
    let cells: Vec<Value> = notebook
        .cells()
        .iter()
        .map(|cell| {
            let mut out = cell.extra.clone();
            out.insert("id".into(), json!(cell.id));
            out.insert(
                "cell_type".into(),
                json!(match cell.cell_type {
                    CellType::Code => "code",
                    CellType::Markdown => "markdown",
                    CellType::Raw => "raw",
                }),
            );
            out.insert("source".into(), lines(&cell.input));
            out.insert("metadata".into(), Value::Object(cell.metadata.clone()));
            if cell.cell_type == CellType::Code {
                let count = cell.output.as_ref().and_then(|o| o.execution_count);
                out.insert("execution_count".into(), json!(count));
                out.insert("outputs".into(), Value::Array(cell.output.as_ref().map(write_outputs).unwrap_or_default()));
            }
            Value::Object(out)
        })
        .collect();
    let file = json!({
        "nbformat": 4,
        "nbformat_minor": 5,
        "metadata": metadata,
        "cells": cells,
    });
    // One-space indent and a final newline, as Jupyter writes
    let mut bytes = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut bytes, formatter);
    serde::Serialize::serialize(&file, &mut serializer).expect("a JSON value always serializes");
    let mut text = String::from_utf8(bytes).expect("serde_json writes UTF-8");
    text.push('\n');
    text
}

/// nbformat's multiline text: one string per line, each keeping its `\n`.
fn lines(text: &str) -> Value {
    json!(text.split_inclusive('\n').collect::<Vec<_>>())
}

fn write_outputs(output: &CellOutput) -> Vec<Value> {
    let mut outputs = Vec::new();
    // A Maxima error with no separate message is in the print text: write
    // it once, as the error
    let error_text = match (&output.error, output.is_error) {
        (Some(message), true) => Some(message.clone()),
        (None, true) => Some(output.text_output.trim().to_string()),
        _ => None,
    };
    let print_is_error = output.is_error && output.error.is_none();
    if !output.text_output.is_empty() && !print_is_error {
        outputs.push(json!({ "output_type": "stream", "name": "stdout", "text": lines(&output.text_output) }));
    }
    if output.latex.is_some() || output.text_plain.is_some() {
        let mut data = Map::new();
        if let Some(latex) = &output.latex {
            data.insert(LATEX.into(), lines(latex));
        }
        if let Some(plain) = &output.text_plain {
            data.insert(PLAIN.into(), lines(plain));
        }
        outputs.push(json!({
            "output_type": "execute_result",
            "execution_count": output.execution_count,
            "data": data,
            "metadata": {},
        }));
    }
    for (value, mime, description) in [
        (&output.plot_data, PLOTLY, "<Plotly figure>"),
        (&output.plot_svg, SVG, "<SVG plot>"),
        (&output.image_png, PNG, "<PNG image>"),
    ] {
        if let Some(value) = value {
            // PNG is base64: one string, as Jupyter writes it
            let content = if mime == PNG { json!(value) } else { lines(value) };
            outputs.push(json!({
                "output_type": "display_data",
                "data": { mime: content, PLAIN: [description] },
                "metadata": {},
            }));
        }
    }
    if let Some(message) = error_text {
        outputs.push(json!({ "output_type": "error", "ename": "MaximaError", "evalue": message, "traceback": [] }));
    }
    outputs.extend(output.other_outputs.iter().cloned());
    outputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::NotebookCommand;

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
        nb.apply(NotebookCommand::LoadNotebook { file: file.clone().into() }).unwrap();
        (file, nb)
    }

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn cell<'a>(nb: &'a Notebook, input: &str) -> &'a crate::notebook::Cell {
        nb.cells().iter().find(|c| c.input == input).unwrap_or_else(|| panic!("no cell {input:?}"))
    }

    #[test]
    fn the_spec_example_round_trips_unchanged() {
        let (file, nb) = load("spec-example.macnb");
        assert!(!file.ids_replaced);
        assert_eq!(json(&write(&nb)), json(&fixture("spec-example.macnb")));
    }

    #[test]
    fn written_files_are_nbformat_4_5_and_end_with_a_newline() {
        let (_, nb) = load("aximar-window-before-s1.macnb");
        let text = write(&nb);
        assert!(text.ends_with("}\n"));
        let v = json(&text);
        assert_eq!((v["nbformat"].as_u64(), v["nbformat_minor"].as_u64()), (Some(4), Some(5)));
        assert!(v["cells"].as_array().unwrap().iter().all(|c| c["id"].is_string()));
    }

    #[test]
    fn cell_ids_follow_the_rule() {
        let file = read(&fixture("cell-ids.macnb")).unwrap();
        let ids: Vec<&str> = file.cells.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids[0], "a1");
        assert_ne!(ids[1], "a1", "a duplicate gets a new ID");
        assert_eq!(ids[2], "b-2_c");
        for replaced in &ids[3..] {
            assert!(valid_id(replaced) && replaced.len() == 21, "{replaced}");
        }
        assert!(file.ids_replaced);
        let unique: HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
    }

    #[test]
    fn every_loaded_output_is_stale() {
        for name in ["spec-example.macnb", "vscode-extension-before-s1.macnb", "aximar-mcp-before-s1.macnb"] {
            let file = read(&fixture(name)).unwrap();
            for out in file.cells.iter().filter_map(|c| c.output.as_ref()) {
                assert!(out.stale, "{name}");
            }
        }
    }

    #[test]
    fn the_vscode_file_reads_plots_as_plots_and_keeps_errors() {
        let (_, nb) = load("vscode-extension-before-s1.macnb");
        // The text/plain copy of the SVG source is not shown (#6)
        let plot = cell(&nb, "plot2d(x^2, [x, 0, 1])$").output.as_ref().unwrap();
        assert!(plot.plot_svg.is_some());
        assert!(plot.text_output.is_empty(), "{:?}", plot.text_output);
        let plotly = cell(&nb, "plot2d(x, [x, 0, 1], [plot_format, plotly])$").output.as_ref().unwrap();
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
        assert!(cell(&nb, "plot2d(x^2, [x, 0, 1])$").output.as_ref().unwrap().plot_svg.is_some());
        // The failed cell was saved with no outputs (#7)
        assert!(cell(&nb, "1/0;").output.is_none());
        let (_, nb) = load("aximar-window-before-s1.macnb");
        assert_eq!(cell(&nb, "print(\"working\")$\nintegrate(x, x);").output.as_ref().unwrap().text_output, "working");
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
        nb.apply(NotebookCommand::SetCellOutput { cell_id: id, output, raw_output: Vec::new() }).unwrap();
        let v = json(&write(&nb));
        let outputs = v["cells"][0]["outputs"].as_array().unwrap();
        assert_eq!(outputs.len(), 1, "{outputs:?}");
        assert_eq!(outputs[0]["output_type"], "error");
        assert_eq!(outputs[0]["evalue"], "incorrect syntax: x is not an infix operator");
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
        nb.apply(NotebookCommand::LoadNotebook { file: file.into() }).unwrap();
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
        nb.apply(NotebookCommand::LoadNotebook { file: file.into() }).unwrap();
        let v = json(&write(&nb));
        assert_eq!(v["cells"][0]["attachments"]["x.png"]["image/png"], "iVBORw0KGgo=");
    }

    #[test]
    fn other_versions_are_refused() {
        assert!(matches!(read(r#"{"nbformat": 3, "cells": []}"#), Err(FormatError::Version(3))));
        assert!(matches!(read("not json"), Err(FormatError::Json(_))));
        assert!(matches!(read("[]"), Err(FormatError::Shape(_))));
    }

    #[test]
    fn loaded_errors_get_their_friendly_explanation() {
        let mut file = read(&fixture("vscode-extension-before-s1.macnb")).unwrap();
        explain_errors(&mut file, &Catalog::load(), &PackageCatalog::load());
        let failed = file.cells.iter().find(|c| c.input == "1/0;").unwrap();
        let info = failed.output.as_ref().unwrap().error_info.as_ref().expect("an explanation");
        assert_eq!(info.title, "Division by Zero");
        // Outputs that aren't errors get none
        assert!(file.cells.iter().filter(|c| c.input != "1/0;").all(|c| c.output.as_ref().is_none_or(|o| o.error_info.is_none())));
    }
}
