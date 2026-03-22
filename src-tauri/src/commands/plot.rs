use std::fs;

#[tauri::command]
pub fn write_plot_svg(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| format!("Failed to save SVG: {e}"))
}
