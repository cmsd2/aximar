use std::fs;
use std::path::Path;

use crate::notebooks::types::Notebook;

pub fn read_notebook(path: &str) -> Result<Notebook, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid notebook format: {e}"))
}

pub fn write_notebook(path: &str, notebook: &Notebook) -> Result<(), String> {
    let json = serde_json::to_string_pretty(notebook)
        .map_err(|e| format!("Failed to serialize notebook: {e}"))?;
    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory: {e}"))?;
    }
    fs::write(path, json).map_err(|e| format!("Failed to write {path}: {e}"))
}
