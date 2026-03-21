use crate::notebooks::data;
use crate::notebooks::types::{Notebook, TemplateSummary};

#[tauri::command]
pub fn list_templates() -> Vec<TemplateSummary> {
    data::list_templates()
}

#[tauri::command]
pub fn get_template(id: String) -> Option<Notebook> {
    data::get_template(&id)
}
