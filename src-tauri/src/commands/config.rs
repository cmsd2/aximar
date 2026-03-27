use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use tokio_util::sync::CancellationToken;

use aximar_core::error::AppError;
use aximar_core::maxima::backend::{decode_wsl_output, Backend, BackendKind, ContainerEngine};
use aximar_core::maxima::noconsole::hide_console_window;

use crate::state::AppState;
use crate::tauri_output::{emit_app_log, AppLogEvent};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    Auto,
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CellStyle {
    Card,
    Bracket,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self::Bracket
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutocompleteMode {
    Hint,
    Snippet,
    ActiveHint,
}

impl Default for AutocompleteMode {
    fn default() -> Self {
        Self::ActiveHint
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarkdownFont {
    SansSerif,
    Serif,
    ComputerModern,
    Mono,
}

impl Default for MarkdownFont {
    fn default() -> Self {
        Self::SansSerif
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarkdownIndent {
    Flush,
    Aligned,
}

impl Default for MarkdownIndent {
    fn default() -> Self {
        Self::Flush
    }
}

fn default_font_size() -> u32 {
    14
}

fn default_eval_timeout() -> u64 {
    30
}

fn default_print_font_size() -> u32 {
    12
}

fn default_print_margin_top() -> u32 { 15 }
fn default_print_margin_bottom() -> u32 { 15 }
fn default_print_margin_left() -> u32 { 24 }
fn default_print_margin_right() -> u32 { 24 }

fn default_docker_image() -> String { String::new() }
fn default_wsl_distro() -> String { String::new() }
fn default_mcp_listen_address() -> String { "127.0.0.1:19542".to_string() }

fn generate_mcp_token() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn default_mcp_token() -> String {
    generate_mcp_token()
}

#[derive(Debug, Serialize, Deserialize)]
struct AppConfig {
    #[serde(default)]
    theme: Theme,
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
    #[serde(default)]
    cell_style: CellStyle,
    #[serde(default = "default_print_font_size")]
    print_font_size: u32,
    #[serde(default)]
    autocomplete_mode: AutocompleteMode,
    #[serde(default)]
    markdown_font: MarkdownFont,
    #[serde(default)]
    markdown_indent: MarkdownIndent,
    #[serde(default = "default_print_margin_top")]
    print_margin_top: u32,
    #[serde(default = "default_print_margin_bottom")]
    print_margin_bottom: u32,
    #[serde(default = "default_print_margin_left")]
    print_margin_left: u32,
    #[serde(default = "default_print_margin_right")]
    print_margin_right: u32,
    #[serde(default)]
    backend: BackendKind,
    #[serde(default = "default_docker_image")]
    docker_image: String,
    #[serde(default = "default_wsl_distro")]
    wsl_distro: String,
    #[serde(default)]
    container_engine: ContainerEngine,
    #[serde(default)]
    mcp_enabled: bool,
    #[serde(default = "default_mcp_listen_address")]
    mcp_listen_address: String,
    #[serde(default = "default_mcp_token")]
    mcp_token: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            has_seen_welcome: false,
            maxima_path: None,
            font_size: default_font_size(),
            eval_timeout: default_eval_timeout(),
            variables_open: false,
            cell_style: CellStyle::default(),
            print_font_size: default_print_font_size(),
            autocomplete_mode: AutocompleteMode::default(),
            markdown_font: MarkdownFont::default(),
            markdown_indent: MarkdownIndent::default(),
            print_margin_top: default_print_margin_top(),
            print_margin_bottom: default_print_margin_bottom(),
            print_margin_left: default_print_margin_left(),
            print_margin_right: default_print_margin_right(),
            backend: BackendKind::default(),
            docker_image: default_docker_image(),
            wsl_distro: default_wsl_distro(),
            container_engine: ContainerEngine::default(),
            mcp_enabled: false,
            mcp_listen_address: default_mcp_listen_address(),
            mcp_token: default_mcp_token(),
        }
    }
}

impl AppConfig {
    fn validated(mut self) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
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
        if self.mcp_listen_address.is_empty()
            || self.mcp_listen_address.parse::<std::net::SocketAddr>().is_err()
        {
            warnings.push(format!(
                "Invalid mcp_listen_address \"{}\", reset to {}",
                self.mcp_listen_address,
                default_mcp_listen_address()
            ));
            self.mcp_listen_address = default_mcp_listen_address();
        }
        if self.mcp_token.is_empty() {
            self.mcp_token = generate_mcp_token();
        }
        (self, warnings)
    }
}

/// Public config returned to the frontend (excludes internal fields like has_seen_welcome)
#[derive(Debug, Serialize, Deserialize)]
pub struct PublicConfig {
    pub theme: Theme,
    pub maxima_path: Option<String>,
    pub font_size: u32,
    pub print_font_size: u32,
    pub eval_timeout: u64,
    pub variables_open: bool,
    pub cell_style: CellStyle,
    pub autocomplete_mode: AutocompleteMode,
    pub markdown_font: MarkdownFont,
    pub markdown_indent: MarkdownIndent,
    pub print_margin_top: u32,
    pub print_margin_bottom: u32,
    pub print_margin_left: u32,
    pub print_margin_right: u32,
    pub backend: BackendKind,
    pub docker_image: String,
    pub wsl_distro: String,
    pub container_engine: ContainerEngine,
    pub mcp_enabled: bool,
    pub mcp_listen_address: String,
    pub mcp_token: String,
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
    pub theme: Option<Theme>,
    pub maxima_path: Option<Option<String>>,
    pub font_size: Option<u32>,
    pub print_font_size: Option<u32>,
    pub eval_timeout: Option<u64>,
    pub variables_open: Option<bool>,
    pub cell_style: Option<CellStyle>,
    pub autocomplete_mode: Option<AutocompleteMode>,
    pub markdown_font: Option<MarkdownFont>,
    pub markdown_indent: Option<MarkdownIndent>,
    pub print_margin_top: Option<u32>,
    pub print_margin_bottom: Option<u32>,
    pub print_margin_left: Option<u32>,
    pub print_margin_right: Option<u32>,
    pub backend: Option<BackendKind>,
    pub docker_image: Option<String>,
    pub wsl_distro: Option<String>,
    pub container_engine: Option<ContainerEngine>,
    pub mcp_enabled: Option<bool>,
    pub mcp_listen_address: Option<String>,
    pub mcp_token: Option<String>,
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
pub async fn get_theme(app: tauri::AppHandle) -> Result<Theme, AppError> {
    let (config, _) = read_config(&app)?;
    Ok(config.theme)
}

#[tauri::command]
pub async fn set_theme(app: tauri::AppHandle, theme: Theme) -> Result<(), AppError> {
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
            backend: config.backend,
            docker_image: config.docker_image,
            wsl_distro: config.wsl_distro,
            container_engine: config.container_engine,
            mcp_enabled: config.mcp_enabled,
            mcp_listen_address: config.mcp_listen_address,
            mcp_token: config.mcp_token,
        },
        warnings,
    })
}

#[tauri::command]
pub async fn set_config(app: tauri::AppHandle, updates: ConfigUpdate) -> Result<(), AppError> {
    let mcp_changed = updates.mcp_enabled.is_some() || updates.mcp_listen_address.is_some() || updates.mcp_token.is_some();

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
    if let Some(backend) = updates.backend { config.backend = backend; }
    if let Some(docker_image) = updates.docker_image { config.docker_image = docker_image; }
    if let Some(wsl_distro) = updates.wsl_distro { config.wsl_distro = wsl_distro; }
    if let Some(container_engine) = updates.container_engine { config.container_engine = container_engine; }
    if let Some(mcp_enabled) = updates.mcp_enabled { config.mcp_enabled = mcp_enabled; }
    if let Some(mcp_listen_address) = updates.mcp_listen_address { config.mcp_listen_address = mcp_listen_address; }
    if let Some(mcp_token) = updates.mcp_token { config.mcp_token = mcp_token; }
    write_config(&app, &config)?;

    // Handle MCP server lifecycle changes
    if mcp_changed {
        let state = app.state::<AppState>();
        let controller = state.mcp_controller.clone();

        if controller.is_running().await {
            emit_app_log(&state.app_handle, &state.app_log, "info", "MCP server stopping...", "mcp");
            controller.stop().await;
        }

        if config.mcp_enabled {
            emit_app_log(&state.app_handle, &state.app_log, "info", &format!("MCP server starting on {}...", config.mcp_listen_address), "mcp");
            let mcp_state = AppState {
                registry: state.registry.clone(),
                catalog: state.catalog.clone(),
                docs: state.docs.clone(),
                packages: state.packages.clone(),
                app_handle: state.app_handle.clone(),
                mcp_controller: state.mcp_controller.clone(),
                app_log: state.app_log.clone(),
            };
            let ct = CancellationToken::new();
            controller.set_running(ct.clone()).await;
            tokio::spawn(crate::mcp::start_mcp_server(
                mcp_state,
                config.mcp_listen_address,
                config.mcp_token,
                ct,
            ));
        }
    }

    Ok(())
}

/// Read a single config value by field name (for internal use by other commands)
pub fn read_eval_timeout(app: &tauri::AppHandle) -> u64 {
    read_config(app).map(|(c, _)| c.eval_timeout).unwrap_or(default_eval_timeout())
}

pub fn read_maxima_path(app: &tauri::AppHandle) -> Option<String> {
    read_config(app).ok().map(|(c, _)| c.maxima_path).flatten()
}

#[tauri::command]
pub async fn list_wsl_distros() -> Result<Vec<String>, AppError> {
    let mut cmd = tokio::process::Command::new("wsl");
    cmd.args(["-l", "-q"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    hide_console_window(&mut cmd);
    let output = cmd.output().await
        .map_err(|e| AppError::ProcessStartFailed(format!("Failed to list WSL distros: {}", e)))?;

    let text = decode_wsl_output(&output.stdout);
    let distros: Vec<String> = text
        .lines()
        .map(|l| l.trim().trim_end_matches('\0'))
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();
    Ok(distros)
}

#[tauri::command]
pub async fn check_wsl_maxima(distro: String) -> Result<Option<String>, AppError> {
    let mut cmd = tokio::process::Command::new("wsl");
    if !distro.is_empty() {
        cmd.args(["-d", &distro]);
    }
    cmd.args(["--", "which", "maxima"]);
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    hide_console_window(&mut cmd);
    let output = cmd.output().await
        .map_err(|e| AppError::ProcessStartFailed(format!("Failed to check maxima in WSL: {}", e)))?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

/// MCP startup config returned by `read_mcp_config`.
pub struct McpConfig {
    pub enabled: bool,
    pub listen_address: String,
    pub token: String,
}

/// Read MCP-related config in a single pass and persist any generated defaults
/// (like `mcp_token`) back to disk. This avoids token mismatches caused by
/// `default_mcp_token()` generating a different random value on each call.
pub fn read_mcp_config(app: &tauri::AppHandle) -> McpConfig {
    let (config, _) = read_config(app).unwrap_or_else(|_| (AppConfig::default(), Vec::new()));
    // Persist so that generated defaults (especially mcp_token) are saved to disk.
    let _ = write_config(app, &config);
    McpConfig {
        enabled: config.mcp_enabled,
        listen_address: config.mcp_listen_address,
        token: config.mcp_token,
    }
}

#[derive(Debug, Serialize)]
pub struct ClaudeMcpStatus {
    pub installed: bool,
    pub configured: bool,
}

#[tauri::command]
pub async fn claude_mcp_status() -> Result<ClaudeMcpStatus, AppError> {
    let mut cmd = tokio::process::Command::new("claude");
    cmd.args(["mcp", "get", "aximar"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    hide_console_window(&mut cmd);

    match cmd.output().await {
        Ok(output) => Ok(ClaudeMcpStatus {
            installed: true,
            configured: output.status.success(),
        }),
        Err(_) => Ok(ClaudeMcpStatus {
            installed: false,
            configured: false,
        }),
    }
}

#[tauri::command]
pub async fn claude_mcp_configure(url: String, token: String) -> Result<String, AppError> {
    // Remove existing entry (ignore errors — may not exist)
    let mut rm_cmd = tokio::process::Command::new("claude");
    rm_cmd
        .args(["mcp", "remove", "aximar"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    hide_console_window(&mut rm_cmd);
    let _ = rm_cmd.output().await;

    // Add new entry
    let header = format!("Authorization: Bearer {token}");
    let mut add_cmd = tokio::process::Command::new("claude");
    add_cmd
        .args([
            "mcp", "add",
            "--transport", "http",
            "--header", &header,
            "--", "aximar", &url,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    hide_console_window(&mut add_cmd);

    let output = add_cmd.output().await.map_err(|e| {
        AppError::ProcessStartFailed(format!("Failed to run claude CLI: {e}"))
    })?;

    if output.status.success() {
        Ok("Claude Code MCP configured successfully".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(AppError::ProcessStartFailed(format!(
            "claude mcp add failed: {stderr}"
        )))
    }
}

#[derive(Debug, Serialize)]
pub struct CodexMcpStatus {
    pub installed: bool,
    pub configured: bool,
}

#[tauri::command]
pub async fn codex_mcp_status() -> Result<CodexMcpStatus, AppError> {
    let mut cmd = tokio::process::Command::new("codex");
    cmd.args(["mcp", "get", "aximar"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    hide_console_window(&mut cmd);

    match cmd.output().await {
        Ok(output) => Ok(CodexMcpStatus {
            installed: true,
            configured: output.status.success(),
        }),
        Err(_) => Ok(CodexMcpStatus {
            installed: false,
            configured: false,
        }),
    }
}

#[tauri::command]
pub async fn codex_mcp_configure(url: String, token: String) -> Result<String, AppError> {
    use toml_edit::{DocumentMut, InlineTable, value};

    // Locate config file
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| AppError::ProcessStartFailed("Could not determine home directory".into()))?;
    let config_dir = std::path::Path::new(&home).join(".codex");
    let config_path = config_dir.join("config.toml");

    // Read and parse existing config (or start with empty document)
    let content = if config_path.exists() {
        tokio::fs::read_to_string(&config_path).await.unwrap_or_default()
    } else {
        String::new()
    };
    let mut doc = content.parse::<DocumentMut>().map_err(|e| {
        AppError::ProcessStartFailed(format!("Failed to parse config.toml: {e}"))
    })?;

    // Ensure [mcp_servers] table exists
    if !doc.contains_table("mcp_servers") {
        doc["mcp_servers"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    // Build [mcp_servers.aximar] entry
    let servers = doc["mcp_servers"].as_table_mut().expect("mcp_servers is a table");
    let mut entry = toml_edit::Table::new();
    entry["url"] = value(&url);
    let mut headers = InlineTable::new();
    headers.insert("Authorization", format!("Bearer {token}").into());
    entry["http_headers"] = toml_edit::Item::Value(toml_edit::Value::InlineTable(headers));
    servers["aximar"] = toml_edit::Item::Table(entry);

    // Ensure directory exists and write config
    tokio::fs::create_dir_all(&config_dir).await.ok();
    tokio::fs::write(&config_path, doc.to_string()).await.map_err(|e| {
        AppError::ProcessStartFailed(format!("Failed to write config.toml: {e}"))
    })?;

    Ok("Codex MCP configured successfully".to_string())
}

#[derive(Debug, Serialize)]
pub struct GeminiMcpStatus {
    pub installed: bool,
    pub configured: bool,
}

#[tauri::command]
pub async fn gemini_mcp_status() -> Result<GeminiMcpStatus, AppError> {
    // Check if gemini CLI is installed
    let mut cmd = tokio::process::Command::new("gemini");
    cmd.args(["--version"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    hide_console_window(&mut cmd);

    let installed = match cmd.output().await {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };

    if !installed {
        return Ok(GeminiMcpStatus { installed: false, configured: false });
    }

    // Check if aximar is configured in ~/.gemini/settings.json
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let settings_path = std::path::Path::new(&home).join(".gemini").join("settings.json");

    let configured = if settings_path.exists() {
        match tokio::fs::read_to_string(&settings_path).await {
            Ok(content) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    json.get("mcpServers")
                        .and_then(|s| s.get("aximar"))
                        .is_some()
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    } else {
        false
    };

    Ok(GeminiMcpStatus { installed, configured })
}

#[tauri::command]
pub async fn gemini_mcp_configure(url: String, token: String) -> Result<String, AppError> {
    // Locate settings file
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| AppError::ProcessStartFailed("Could not determine home directory".into()))?;
    let config_dir = std::path::Path::new(&home).join(".gemini");
    let settings_path = config_dir.join("settings.json");

    // Read existing settings or start with empty object
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = tokio::fs::read_to_string(&settings_path).await.unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Ensure mcpServers object exists
    if !settings.get("mcpServers").is_some() {
        settings["mcpServers"] = serde_json::json!({});
    }

    // Add/update aximar entry
    settings["mcpServers"]["aximar"] = serde_json::json!({
        "httpUrl": url,
        "headers": {
            "Authorization": format!("Bearer {token}")
        }
    });

    // Write back with pretty formatting
    let output = serde_json::to_string_pretty(&settings).map_err(|e| {
        AppError::ProcessStartFailed(format!("Failed to serialize settings: {e}"))
    })?;

    tokio::fs::create_dir_all(&config_dir).await.ok();
    tokio::fs::write(&settings_path, output).await.map_err(|e| {
        AppError::ProcessStartFailed(format!("Failed to write settings.json: {e}"))
    })?;

    Ok("Gemini CLI MCP configured successfully".to_string())
}

pub fn read_backend(app: &tauri::AppHandle) -> Backend {
    let config = read_config(app).map(|(c, _)| c).unwrap_or_default();
    Backend::from_config(config.backend, &config.docker_image, &config.wsl_distro, config.container_engine)
}

/// Drain all buffered app log entries. Called by the frontend on mount to
/// replay any logs emitted before the event listener was ready.
#[tauri::command]
pub async fn get_buffered_logs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AppLogEvent>, AppError> {
    Ok(state.app_log.drain())
}
