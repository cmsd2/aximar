//! Reads and writes Maxima notebook files (`.macnb`, Jupyter nbformat 4.5)
//! the way Aximar does.  `docs/notebook-format.md` in the Aximar
//! repository describes the format; this crate is the one implementation
//! of it, used by Aximar (desktop app, MCP server, batch runs) and, built
//! as WebAssembly, by the VS Code extension through the npm package of the
//! same name.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[cfg(feature = "wasm")]
mod wasm;

/// A notebook as stored in a file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
pub struct NotebookFile {
    /// Notebook metadata, kept and written back.  The writer sets
    /// `kernelspec`.
    #[cfg_attr(feature = "wasm", tsify(type = "Record<string, unknown>"))]
    #[serde(default)]
    pub metadata: Map<String, Value>,
    pub cells: Vec<Cell>,
    /// Set by [`read`]: some cells had no usable ID and were given new
    /// ones, so the notebook no longer matches its file.  Not written.
    #[serde(default)]
    pub ids_replaced: bool,
}

/// One cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
pub struct Cell {
    pub id: String,
    pub cell_type: CellType,
    /// The cell's source, as one string.
    pub input: String,
    #[cfg_attr(feature = "wasm", tsify(type = "Record<string, unknown>"))]
    #[serde(default)]
    pub metadata: Map<String, Value>,
    /// Fields of the cell this crate doesn't read, such as `attachments`,
    /// written back unchanged.
    #[cfg_attr(feature = "wasm", tsify(type = "Record<string, unknown>"))]
    #[serde(default)]
    pub extra: Map<String, Value>,
    /// Code cells only.
    #[cfg_attr(feature = "wasm", tsify(type = "Output | null"))]
    #[serde(default)]
    pub output: Option<Output>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
pub enum CellType {
    Code,
    Markdown,
    /// Raw cells, and any cell type a later nbformat adds: kept as text,
    /// never run.
    Raw,
}

/// A code cell's output: everything one run shows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[cfg_attr(feature = "wasm", derive(tsify_next::Tsify))]
pub struct Output {
    /// Print text and warnings.
    pub text_output: String,
    /// The final value as LaTeX.
    #[cfg_attr(feature = "wasm", tsify(type = "string | null"))]
    pub latex: Option<String>,
    /// The final value in Maxima's one-line form, as `string()` writes it.
    #[cfg_attr(feature = "wasm", tsify(type = "string | null"))]
    pub text_plain: Option<String>,
    /// An SVG plot.
    #[cfg_attr(feature = "wasm", tsify(type = "string | null"))]
    pub plot_svg: Option<String>,
    /// A Plotly figure, as JSON.
    #[cfg_attr(feature = "wasm", tsify(type = "string | null"))]
    pub plot_data: Option<String>,
    /// A PNG image, base64.
    #[cfg_attr(feature = "wasm", tsify(type = "string | null"))]
    pub image_png: Option<String>,
    /// The error message.  An error may also be in `text_output` with no
    /// separate message.
    #[cfg_attr(feature = "wasm", tsify(type = "string | null"))]
    pub error: Option<String>,
    pub is_error: bool,
    /// The gutter number shown with the output.
    #[cfg_attr(feature = "wasm", tsify(type = "number | null"))]
    pub execution_count: Option<u32>,
    /// Read from a file: from a session that no longer exists.  Not written.
    pub stale: bool,
    /// Outputs of kinds this crate doesn't show, written back unchanged.
    #[cfg_attr(feature = "wasm", tsify(type = "unknown[]"))]
    pub other_outputs: Vec<Value>,
}

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

/// Fields of a cell this crate reads; anything else is kept in `extra`.
const CELL_FIELDS: &[&str] = &[
    "id",
    "cell_type",
    "source",
    "metadata",
    "execution_count",
    "outputs",
];

// ── Cell IDs ─────────────────────────────────────────────────────────

/// nbformat 4.5's rule for a cell ID: 1 to 64 letters, digits, `-` or `_`.
pub fn valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A new random cell ID: 21 characters of `A-Za-z0-9_-`, as nanoid makes.
pub fn new_id() -> String {
    const ALPHABET: &[u8; 64] = b"_-0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut bytes = [0u8; 21];
    getrandom::getrandom(&mut bytes).expect("the system has a random source");
    bytes
        .iter()
        .map(|b| ALPHABET[(b & 63) as usize] as char)
        .collect()
}

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
    let metadata = root
        .get("metadata")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut seen = HashSet::new();
    let mut ids_replaced = false;
    let mut cells = Vec::new();
    for raw in root
        .get("cells")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(cell) = raw.as_object() else {
            return Err(FormatError::Shape("a cell isn't an object".into()));
        };
        let id = match cell.get("id").and_then(Value::as_str) {
            Some(id) if valid_id(id) && seen.insert(id.to_string()) => id.to_string(),
            _ => {
                ids_replaced = true;
                let id = new_id();
                seen.insert(id.clone());
                id
            }
        };
        let cell_type = match cell.get("cell_type").and_then(Value::as_str) {
            Some("code") => CellType::Code,
            Some("markdown") => CellType::Markdown,
            _ => CellType::Raw,
        };
        let output = match cell_type {
            CellType::Code => read_outputs(
                cell.get("outputs")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
                cell.get("execution_count")
                    .and_then(Value::as_u64)
                    .map(|n| n as u32),
            ),
            _ => None,
        };
        let extra = cell
            .iter()
            .filter(|(key, _)| !CELL_FIELDS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        cells.push(Cell {
            id,
            cell_type,
            input: text(cell.get("source")),
            metadata: cell
                .get("metadata")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
            extra,
            output,
        });
    }
    Ok(NotebookFile {
        metadata,
        cells,
        ids_replaced,
    })
}

/// A string, or a list of strings joined (nbformat's multiline text).
pub fn text(value: Option<&Value>) -> String {
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

fn read_outputs(outputs: &[Value], execution_count: Option<u32>) -> Option<Output> {
    if outputs.is_empty() {
        return None;
    }
    let mut out = Output {
        execution_count,
        stale: true,
        ..Default::default()
    };
    let mut others = Vec::new();
    for output in outputs {
        match output.get("output_type").and_then(Value::as_str) {
            Some("stream") => out.text_output.push_str(&text(output.get("text"))),
            Some("error") => {
                out.is_error = true;
                let message = text(output.get("evalue"));
                out.error = Some(if message.is_empty() {
                    text(output.get("ename"))
                } else {
                    message
                });
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
                        // The VS Code extension up to v0.3.0 copied the
                        // LaTeX into text/plain: that's no one-line form
                        out.text_plain = get(PLAIN).filter(|plain| *plain != v);
                        out.latex = Some(v);
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

// ── Writing ──────────────────────────────────────────────────────────

/// Write a notebook as nbformat 4.5 JSON, with keys in order, a one-space
/// indent and a final newline, as Jupyter writes.
pub fn write(notebook: &NotebookFile) -> String {
    let mut metadata = notebook.metadata.clone();
    metadata.insert(
        "kernelspec".into(),
        json!({ "name": "maxima", "display_name": "Maxima", "language": "maxima" }),
    );
    let cells: Vec<Value> = notebook
        .cells
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
                out.insert(
                    "outputs".into(),
                    Value::Array(cell.output.as_ref().map(write_outputs).unwrap_or_default()),
                );
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
    let mut bytes = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut bytes, formatter);
    file.serialize(&mut serializer)
        .expect("a JSON value always serializes");
    let mut text = String::from_utf8(bytes).expect("serde_json writes UTF-8");
    text.push('\n');
    text
}

/// nbformat's multiline text: one string per line, each keeping its `\n`.
fn lines(text: &str) -> Value {
    json!(text.split_inclusive('\n').collect::<Vec<_>>())
}

fn write_outputs(output: &Output) -> Vec<Value> {
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
            let content = if mime == PNG {
                json!(value)
            } else {
                lines(value)
            };
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
mod tests;
