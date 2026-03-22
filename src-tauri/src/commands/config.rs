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

fn default_cell_style() -> String {
    "bracket".to_string()
}

fn default_print_font_size() -> u32 {
    12
}

fn default_autocomplete_mode() -> String {
    "active-hint".to_string()
}

fn default_markdown_font() -> String {
    "sans-serif".to_string()
}

fn default_markdown_indent() -> String {
    "flush".to_string()
}

fn default_print_margin_top() -> u32 { 15 }
fn default_print_margin_bottom() -> u32 { 15 }
fn default_print_margin_left() -> u32 { 24 }
fn default_print_margin_right() -> u32 { 24 }

#[derive(Debug, Serialize, Deserialize)]
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
    #[serde(default = "default_cell_style")]
    cell_style: String,
    #[serde(default = "default_print_font_size")]
    print_font_size: u32,
    #[serde(default = "default_autocomplete_mode")]
    autocomplete_mode: String,
    #[serde(default = "default_markdown_font")]
    markdown_font: String,
    #[serde(default = "default_markdown_indent")]
    markdown_indent: String,
    #[serde(default = "default_print_margin_top")]
    print_margin_top: u32,
    #[serde(default = "default_print_margin_bottom")]
    print_margin_bottom: u32,
    #[serde(default = "default_print_margin_left")]
    print_margin_left: u32,
    #[serde(default = "default_print_margin_right")]
    print_margin_right: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            has_seen_welcome: false,
            maxima_path: None,
            font_size: default_font_size(),
            eval_timeout: default_eval_timeout(),
            variables_open: false,
            cell_style: default_cell_style(),
            print_font_size: default_print_font_size(),
            autocomplete_mode: default_autocomplete_mode(),
            markdown_font: default_markdown_font(),
            markdown_indent: default_markdown_indent(),
            print_margin_top: default_print_margin_top(),
            print_margin_bottom: default_print_margin_bottom(),
            print_margin_left: default_print_margin_left(),
            print_margin_right: default_print_margin_right(),
        }
    }
}

impl AppConfig {
    fn validated(mut self) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        if !matches!(self.theme.as_str(), "auto" | "light" | "dark") {
            warnings.push(format!(
                "Invalid theme '{}', reset to '{}'",
                self.theme,
                default_theme()
            ));
            self.theme = default_theme();
        }
        if !(8..=32).contains(&self.font_size) {
            warnings.push(format!(
                "Invalid font_size {}, reset to {}",
                self.font_size,
                default_font_size()
            ));
            self.font_size = default_font_size();
        }
        if !(8..=32).contains(&self.print_font_size) {
            warnings.push(format!(
                "Invalid print_font_size {}, reset to {}",
                self.print_font_size,
                default_print_font_size()
            ));
            self.print_font_size = default_print_font_size();
        }
        if self.eval_timeout == 0 || self.eval_timeout > 600 {
            warnings.push(format!(
                "Invalid eval_timeout {}, reset to {}",
                self.eval_timeout,
                default_eval_timeout()
            ));
            self.eval_timeout = default_eval_timeout();
        }
        if !matches!(self.cell_style.as_str(), "card" | "bracket") {
            warnings.push(format!(
                "Invalid cell_style '{}', reset to '{}'",
                self.cell_style,
                default_cell_style()
            ));
            self.cell_style = default_cell_style();
        }
        if !matches!(self.autocomplete_mode.as_str(), "hint" | "snippet" | "active-hint") {
            warnings.push(format!(
                "Invalid autocomplete_mode '{}', reset to '{}'",
                self.autocomplete_mode,
                default_autocomplete_mode()
            ));
            self.autocomplete_mode = default_autocomplete_mode();
        }
        if !matches!(self.markdown_font.as_str(), "sans-serif" | "serif" | "computer-modern" | "mono") {
            warnings.push(format!(
                "Invalid markdown_font '{}', reset to '{}'",
                self.markdown_font,
                default_markdown_font()
            ));
            self.markdown_font = default_markdown_font();
        }
        if !matches!(self.markdown_indent.as_str(), "flush" | "aligned") {
            warnings.push(format!(
                "Invalid markdown_indent '{}', reset to '{}'",
                self.markdown_indent,
                default_markdown_indent()
            ));
            self.markdown_indent = default_markdown_indent();
        }
        for (field, val, default_fn) in [
            ("print_margin_top", &mut self.print_margin_top, default_print_margin_top as fn() -> u32),
            ("print_margin_bottom", &mut self.print_margin_bottom, default_print_margin_bottom),
            ("print_margin_left", &mut self.print_margin_left, default_print_margin_left),
            ("print_margin_right", &mut self.print_margin_right, default_print_margin_right),
        ] {
            if *val > 50 {
                warnings.push(format!("Invalid {} {}, reset to {}", field, val, default_fn()));
                *val = default_fn();
            }
        }
        (self, warnings)
    }
}

/// Public config returned to the frontend (excludes internal fields like has_seen_welcome)
#[derive(Debug, Serialize, Deserialize)]
pub struct PublicConfig {
    pub theme: String,
    pub maxima_path: Option<String>,
    pub font_size: u32,
    pub print_font_size: u32,
    pub eval_timeout: u64,
    pub variables_open: bool,
    pub cell_style: String,
    pub autocomplete_mode: String,
    pub markdown_font: String,
    pub markdown_indent: String,
    pub print_margin_top: u32,
    pub print_margin_bottom: u32,
    pub print_margin_left: u32,
    pub print_margin_right: u32,
}

/// Config response with validation warnings
#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub config: PublicConfig,
    pub warnings: Vec<String>,
}

/// Partial config for updates from the frontend
#[derive(Debug, Deserialize)]
pub struct ConfigUpdate {
    pub theme: Option<String>,
    pub maxima_path: Option<Option<String>>,
    pub font_size: Option<u32>,
    pub print_font_size: Option<u32>,
    pub eval_timeout: Option<u64>,
    pub variables_open: Option<bool>,
    pub cell_style: Option<String>,
    pub autocomplete_mode: Option<String>,
    pub markdown_font: Option<String>,
    pub markdown_indent: Option<String>,
    pub print_margin_top: Option<u32>,
    pub print_margin_bottom: Option<u32>,
    pub print_margin_left: Option<u32>,
    pub print_margin_right: Option<u32>,
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let dir = app.path().app_config_dir().map_err(|e| {
        AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    })?;
    Ok(dir.join("config.json"))
}

fn read_config(app: &tauri::AppHandle) -> Result<(AppConfig, Vec<String>), AppError> {
    let path = config_path(app)?;
    if !path.exists() {
        return Ok((AppConfig::default(), Vec::new()));
    }
    let contents = fs::read_to_string(&path)?;
    let config: AppConfig = serde_json::from_str(&contents)
        .unwrap_or_default();
    Ok(config.validated())
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
    let (config, _) = read_config(&app)?;
    Ok(config.theme)
}

#[tauri::command]
pub async fn set_theme(app: tauri::AppHandle, theme: String) -> Result<(), AppError> {
    let (mut config, _) = read_config(&app)?;
    config.theme = theme;
    write_config(&app, &config)?;
    Ok(())
}

#[tauri::command]
pub async fn get_has_seen_welcome(app: tauri::AppHandle) -> Result<bool, AppError> {
    let (config, _) = read_config(&app)?;
    Ok(config.has_seen_welcome)
}

#[tauri::command]
pub async fn set_has_seen_welcome(app: tauri::AppHandle) -> Result<(), AppError> {
    let (mut config, _) = read_config(&app)?;
    config.has_seen_welcome = true;
    write_config(&app, &config)?;
    Ok(())
}

#[tauri::command]
pub async fn get_config(app: tauri::AppHandle) -> Result<ConfigResponse, AppError> {
    let (config, warnings) = read_config(&app)?;
    Ok(ConfigResponse {
        config: PublicConfig {
            theme: config.theme,
            maxima_path: config.maxima_path,
            font_size: config.font_size,
            print_font_size: config.print_font_size,
            eval_timeout: config.eval_timeout,
            variables_open: config.variables_open,
            cell_style: config.cell_style,
            autocomplete_mode: config.autocomplete_mode,
            markdown_font: config.markdown_font,
            markdown_indent: config.markdown_indent,
            print_margin_top: config.print_margin_top,
            print_margin_bottom: config.print_margin_bottom,
            print_margin_left: config.print_margin_left,
            print_margin_right: config.print_margin_right,
        },
        warnings,
    })
}

#[tauri::command]
pub async fn set_config(app: tauri::AppHandle, updates: ConfigUpdate) -> Result<(), AppError> {
    let (mut config, _) = read_config(&app)?;
    if let Some(theme) = updates.theme {
        config.theme = theme;
    }
    if let Some(maxima_path) = updates.maxima_path {
        config.maxima_path = maxima_path;
    }
    if let Some(font_size) = updates.font_size {
        config.font_size = font_size;
    }
    if let Some(print_font_size) = updates.print_font_size {
        config.print_font_size = print_font_size;
    }
    if let Some(eval_timeout) = updates.eval_timeout {
        config.eval_timeout = eval_timeout;
    }
    if let Some(variables_open) = updates.variables_open {
        config.variables_open = variables_open;
    }
    if let Some(cell_style) = updates.cell_style {
        config.cell_style = cell_style;
    }
    if let Some(autocomplete_mode) = updates.autocomplete_mode {
        config.autocomplete_mode = autocomplete_mode;
    }
    if let Some(markdown_font) = updates.markdown_font {
        config.markdown_font = markdown_font;
    }
    if let Some(markdown_indent) = updates.markdown_indent {
        config.markdown_indent = markdown_indent;
    }
    if let Some(v) = updates.print_margin_top { config.print_margin_top = v; }
    if let Some(v) = updates.print_margin_bottom { config.print_margin_bottom = v; }
    if let Some(v) = updates.print_margin_left { config.print_margin_left = v; }
    if let Some(v) = updates.print_margin_right { config.print_margin_right = v; }
    write_config(&app, &config)?;
    Ok(())
}

/// Read a single config value by field name (for internal use by other commands)
pub fn read_eval_timeout(app: &tauri::AppHandle) -> u64 {
    read_config(app).map(|(c, _)| c.eval_timeout).unwrap_or(default_eval_timeout())
}

pub fn read_maxima_path(app: &tauri::AppHandle) -> Option<String> {
    read_config(app).ok().map(|(c, _)| c.maxima_path).flatten()
}
