use aximar_core::notebooks::data;
use aximar_core::notebooks::io;
use aximar_core::notebooks::types::{Notebook, TemplateSummary};
use std::path::Path;

#[tauri::command]
pub fn list_templates() -> Vec<TemplateSummary> {
    data::list_templates()
}

#[tauri::command]
pub fn get_template(id: String) -> Option<Notebook> {
    data::get_template(&id)
}

#[tauri::command]
pub fn save_notebook(path: String, notebook: Notebook) -> Result<(), String> {
    io::write_notebook(&path, &notebook)
}

#[tauri::command]
pub fn open_notebook(path: String) -> Result<Notebook, String> {
    io::read_notebook(&path)
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    std::fs::read_to_string(p).map_err(|e| format!("Failed to read {}: {e}", p.display()))
}
