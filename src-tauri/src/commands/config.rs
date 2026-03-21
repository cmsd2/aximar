use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

use crate::error::AppError;

fn default_theme() -> String {
    "auto".to_string()
}

fn default_font_size() -> u32 {
    14
}

fn default_eval_timeout() -> u64 {
    30
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default)]
    has_seen_welcome: bool,
    #[serde(default)]
    maxima_path: Option<String>,
    #[serde(default = "default_font_size")]
    font_size: u32,
    #[serde(default = "default_eval_timeout")]
    eval_timeout: u64,
    #[serde(default)]
    variables_open: bool,
}

/// Public config returned to the frontend (excludes internal fields like has_seen_welcome)
#[derive(Debug, Serialize, Deserialize)]
pub struct PublicConfig {
    pub theme: String,
    pub maxima_path: Option<String>,
    pub font_size: u32,
    pub eval_timeout: u64,
    pub variables_open: bool,
}

/// Partial config for updates from the frontend
#[derive(Debug, Deserialize)]
pub struct ConfigUpdate {
    pub theme: Option<String>,
    pub maxima_path: Option<Option<String>>,
    pub font_size: Option<u32>,
    pub eval_timeout: Option<u64>,
    pub variables_open: Option<bool>,
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

#[tauri::command]
pub async fn get_has_seen_welcome(app: tauri::AppHandle) -> Result<bool, AppError> {
    let config = read_config(&app)?;
    Ok(config.has_seen_welcome)
}

#[tauri::command]
pub async fn set_has_seen_welcome(app: tauri::AppHandle) -> Result<(), AppError> {
    let mut config = read_config(&app)?;
    config.has_seen_welcome = true;
    write_config(&app, &config)?;
    Ok(())
}

#[tauri::command]
pub async fn get_config(app: tauri::AppHandle) -> Result<PublicConfig, AppError> {
    let config = read_config(&app)?;
    Ok(PublicConfig {
        theme: config.theme,
        maxima_path: config.maxima_path,
        font_size: config.font_size,
        eval_timeout: config.eval_timeout,
        variables_open: config.variables_open,
    })
}

#[tauri::command]
pub async fn set_config(app: tauri::AppHandle, updates: ConfigUpdate) -> Result<(), AppError> {
    let mut config = read_config(&app)?;
    if let Some(theme) = updates.theme {
        config.theme = theme;
    }
    if let Some(maxima_path) = updates.maxima_path {
        config.maxima_path = maxima_path;
    }
    if let Some(font_size) = updates.font_size {
        config.font_size = font_size;
    }
    if let Some(eval_timeout) = updates.eval_timeout {
        config.eval_timeout = eval_timeout;
    }
    if let Some(variables_open) = updates.variables_open {
        config.variables_open = variables_open;
    }
    write_config(&app, &config)?;
    Ok(())
}

/// Read a single config value by field name (for internal use by other commands)
pub fn read_eval_timeout(app: &tauri::AppHandle) -> u64 {
    read_config(app).map(|c| c.eval_timeout).unwrap_or(default_eval_timeout())
}

pub fn read_maxima_path(app: &tauri::AppHandle) -> Option<String> {
    read_config(app).ok().and_then(|c| c.maxima_path)
}
