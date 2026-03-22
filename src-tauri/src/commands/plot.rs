use std::fs;
use std::path::Path;

#[tauri::command]
pub fn write_plot_svg(path: String, content: String) -> Result<(), String> {
    let p = Path::new(&path);

    // Reject paths containing ".." segments to prevent directory traversal
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err("Invalid path: directory traversal not allowed".to_string());
        }
    }

    // Must have .svg extension
    if p.extension().and_then(|e| e.to_str()) != Some("svg") {
        return Err("Invalid path: file must have .svg extension".to_string());
    }

    fs::write(&path, &content).map_err(|e| format!("Failed to save SVG: {e}"))
}
