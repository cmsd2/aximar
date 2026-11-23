//! JavaScript bindings, for the npm package.  The model crosses as plain
//! JSON-shaped objects with camelCase fields; the TypeScript types come
//! from the Rust structs, so the two can't drift.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::NotebookFile;

fn js_error(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

/// Read a notebook file.  Outputs come back stale, and cells with no
/// usable ID get new ones (`idsReplaced`).  Throws on JSON that isn't an
/// nbformat 4 notebook.
#[wasm_bindgen(js_name = read, unchecked_return_type = "NotebookFile")]
pub fn read_notebook(json: &str) -> Result<JsValue, JsError> {
    let file = crate::read(json).map_err(js_error)?;
    file.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(js_error)
}

/// Write a notebook as nbformat 4.5 JSON, the same bytes Aximar writes.
#[wasm_bindgen(js_name = write)]
pub fn write_notebook(
    #[wasm_bindgen(unchecked_param_type = "NotebookFile")] notebook: JsValue,
) -> Result<String, JsError> {
    let file: NotebookFile = serde_wasm_bindgen::from_value(notebook).map_err(js_error)?;
    Ok(crate::write(&file))
}

/// A new random cell ID.
#[wasm_bindgen(js_name = newId)]
pub fn new_id() -> String {
    crate::new_id()
}

/// Whether an ID fits nbformat 4.5's rule.
#[wasm_bindgen(js_name = validId)]
pub fn valid_id(id: &str) -> bool {
    crate::valid_id(id)
}
