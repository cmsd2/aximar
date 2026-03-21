use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default = "default_theme")]
    theme: String,
}

fn default_theme() -> String {
    "auto".to_string()
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let dir = app.path().app_config_dir().map_err(|e| {
        AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    })?;
    Ok(dir.join("config.json"))
}

fn read_config(app: &tauri::AppHandle) -> Result<AppConfig, AppError> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let contents = fs::read_to_string(&path)?;
    serde_json::from_str(&contents).map_err(|_| AppConfig::default()).or(Ok(AppConfig::default()))
}

fn write_config(app: &tauri::AppHandle, config: &AppConfig) -> Result<(), AppError> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(&path, contents)?;
    Ok(())
}

#[tauri::command]
pub async fn get_theme(app: tauri::AppHandle) -> Result<String, AppError> {
    let config = read_config(&app)?;
    Ok(config.theme)
}

#[tauri::command]
pub async fn set_theme(app: tauri::AppHandle, theme: String) -> Result<(), AppError> {
    let mut config = read_config(&app)?;
    config.theme = theme;
    write_config(&app, &config)?;
    Ok(())
}
