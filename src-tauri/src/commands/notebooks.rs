use aximar_core::notebooks::data;
use aximar_core::notebooks::types::TemplateSummary;
use std::path::Path;

#[tauri::command]
pub fn list_templates() -> Vec<TemplateSummary> {
    data::list_templates()
}

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    let p = Path::new(&path);
    std::fs::read_to_string(p).map_err(|e| format!("Failed to read {}: {e}", p.display()))
}
