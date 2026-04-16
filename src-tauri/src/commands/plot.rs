use std::fs;
use std::path::Path;

/// Validate a user-provided file path: no directory traversal, required extension.
fn validate_path(path: &Path, allowed_extensions: &[&str]) -> Result<(), String> {
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err("Invalid path: directory traversal not allowed".to_string());
        }
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !allowed_extensions.contains(&ext) {
        return Err(format!(
            "Invalid path: file must have one of these extensions: {}",
            allowed_extensions.join(", ")
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn write_plot_svg(path: String, content: String) -> Result<(), String> {
    validate_path(Path::new(&path), &["svg"])?;
    fs::write(&path, &content).map_err(|e| format!("Failed to save SVG: {e}"))
}

#[tauri::command]
pub fn write_binary_file(path: String, data: Vec<u8>) -> Result<(), String> {
    validate_path(Path::new(&path), &["png", "jpg", "jpeg", "svg", "pdf"])?;
    fs::write(&path, &data).map_err(|e| format!("Failed to save file: {e}"))
}

#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    validate_path(Path::new(&path), &["tex", "txt", "latex", "json"])?;
    fs::write(&path, &content).map_err(|e| format!("Failed to save file: {e}"))
}

#[tauri::command]
pub fn ensure_directory(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err("Invalid path: directory traversal not allowed".to_string());
        }
    }
    fs::create_dir_all(p).map_err(|e| format!("Failed to create directory: {e}"))
}
