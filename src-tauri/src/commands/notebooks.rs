use crate::notebooks::data;
use crate::notebooks::io;
use crate::notebooks::types::{Notebook, TemplateSummary};

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
