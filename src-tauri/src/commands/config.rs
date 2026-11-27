use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};
use tokio_util::sync::CancellationToken;

use aximar_core::error::AppError;
use aximar_core::maxima::backend::{decode_wsl_output, Backend, BackendKind, ContainerEngine};
use aximar_core::maxima::noconsole::hide_console_window;
use aximar_core::session_ops::SessionSettings;

use crate::i18n;
use crate::state::AppState;
use crate::tauri_output::{emit_app_log, AppLogEvent};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum CellStyle {
    Card,
    #[default]
    Bracket,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum AutocompleteMode {
    Hint,
    Snippet,
    #[default]
    ActiveHint,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum MarkdownFont {
    #[default]
    SansSerif,
    Serif,
    ComputerModern,
    Mono,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[derive(Default)]
pub enum MarkdownIndent {
    #[default]
    Flush,
    Aligned,
}

fn default_show_suggestions() -> bool {
    true
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

fn default_print_margin_top() -> u32 {
    15
}
fn default_print_margin_bottom() -> u32 {
    15
}
fn default_print_margin_left() -> u32 {
    24
}
fn default_print_margin_right() -> u32 {
    24
}

fn default_docker_image() -> String {
    String::new()
}
fn default_wsl_distro() -> String {
    String::new()
}
fn default_mcp_listen_address() -> String {
    "127.0.0.1:19542".to_string()
}

fn generate_mcp_token() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn default_settings_section() -> String {
    "general".to_string()
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
    /// Suggestion chips under results.  A teacher may turn them off so
    /// students write their own Maxima (issue #44).
    #[serde(default = "default_show_suggestions")]
    show_suggestions: bool,
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
    /// The language the user chose, such as `es`; `None` until they do,
    /// which follows the system's language
    #[serde(default)]
    language: Option<String>,
    /// The Settings section shown last, such as `printing`.  The window
    /// knows the sections, and shows General for one it doesn't have.
    #[serde(default = "default_settings_section")]
    settings_section: String,
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
            show_suggestions: default_show_suggestions(),
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
            language: None,
            settings_section: default_settings_section(),
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
            (
                "print_margin_top",
                &mut self.print_margin_top,
                default_print_margin_top as fn() -> u32,
            ),
            (
                "print_margin_bottom",
                &mut self.print_margin_bottom,
                default_print_margin_bottom,
            ),
            (
                "print_margin_left",
                &mut self.print_margin_left,
                default_print_margin_left,
            ),
            (
                "print_margin_right",
                &mut self.print_margin_right,
                default_print_margin_right,
            ),
        ] {
            if *val > 50 {
                warnings.push(format!(
                    "Invalid {} {}, reset to {}",
                    field,
                    val,
                    default_fn()
                ));
                *val = default_fn();
            }
        }
        if self.mcp_listen_address.is_empty()
            || self
                .mcp_listen_address
                .parse::<std::net::SocketAddr>()
                .is_err()
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
        if let Some(language) = &self.language {
            if !i18n::languages().contains(&language.as_str()) {
                warnings.push(format!(
                    "Language \"{language}\" has no translation in this version, so Aximar \
                     follows the system language"
                ));
                self.language = None;
            }
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
    pub show_suggestions: bool,
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
    /// The language in use: the user's choice, or the system's default
    pub language: String,
    /// The languages with a translation, English first
    pub languages: Vec<String>,
    pub settings_section: String,
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
    pub show_suggestions: Option<bool>,
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
    pub language: Option<String>,
    pub settings_section: Option<String>,
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::Io(std::io::Error::other(e.to_string())))?;
    Ok(dir.join("config.json"))
}

/// What the settings file holds.
enum ConfigFile {
    Missing,
    Valid(AppConfig),
    /// The file exists but can't be parsed.  Holds a message for the user.
    Invalid(String),
}

fn parse_config_file(path: &Path, contents: Option<&str>) -> ConfigFile {
    match contents {
        None => ConfigFile::Missing,
        Some(text) => match serde_json::from_str::<AppConfig>(text) {
            Ok(config) => ConfigFile::Valid(config),
            Err(e) => ConfigFile::Invalid(format!(
                "Settings file {} can't be read ({e}). Aximar is using default \
                 settings and won't save changes until the file is fixed or removed.",
                path.display()
            )),
        },
    }
}

fn load_config_file(app: &tauri::AppHandle) -> Result<ConfigFile, AppError> {
    let path = config_path(app)?;
    let contents = if path.exists() {
        Some(fs::read_to_string(&path)?)
    } else {
        None
    };
    Ok(parse_config_file(&path, contents.as_deref()))
}

/// Read settings for display or use.  A file that can't be parsed gives the
/// defaults plus a warning, so the app keeps working.
fn read_config(app: &tauri::AppHandle) -> Result<(AppConfig, Vec<String>), AppError> {
    Ok(config_or_defaults(load_config_file(app)?))
}

fn config_or_defaults(file: ConfigFile) -> (AppConfig, Vec<String>) {
    match file {
        ConfigFile::Missing => (AppConfig::default(), Vec::new()),
        ConfigFile::Valid(config) => config.validated(),
        ConfigFile::Invalid(message) => {
            let (config, mut warnings) = AppConfig::default().validated();
            warnings.insert(0, message);
            (config, warnings)
        }
    }
}

/// Read settings in order to change and save them.  Fails when the file
/// can't be parsed, so that saving never replaces the user's file with
/// defaults.
fn read_config_for_update(app: &tauri::AppHandle) -> Result<AppConfig, AppError> {
    config_for_update(load_config_file(app)?)
}

fn config_for_update(file: ConfigFile) -> Result<AppConfig, AppError> {
    match file {
        ConfigFile::Invalid(message) => Err(AppError::InvalidConfig(message)),
        other => Ok(config_or_defaults(other).0),
    }
}

fn write_config(app: &tauri::AppHandle, config: &AppConfig) -> Result<(), AppError> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(config).map_err(std::io::Error::other)?;
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
    let mut config = read_config_for_update(&app)?;
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
    let mut config = read_config_for_update(&app)?;
    config.has_seen_welcome = true;
    write_config(&app, &config)?;
    Ok(())
}

#[tauri::command]
pub async fn get_config(app: tauri::AppHandle) -> Result<ConfigResponse, AppError> {
    let (config, warnings) = read_config(&app)?;
    let language = config.language();
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
            show_suggestions: config.show_suggestions,
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
            language,
            languages: i18n::languages().iter().map(|l| l.to_string()).collect(),
            settings_section: config.settings_section,
        },
        warnings,
    })
}

#[tauri::command]
pub async fn set_config(app: tauri::AppHandle, updates: ConfigUpdate) -> Result<(), AppError> {
    let mcp_changed = updates.mcp_enabled.is_some()
        || updates.mcp_listen_address.is_some()
        || updates.mcp_token.is_some();

    let mut config = read_config_for_update(&app)?;
    let language_before = config.language();
    if let Some(language) = updates.language {
        if !i18n::languages().contains(&language.as_str()) {
            return Err(AppError::InvalidConfig(format!(
                "Aximar has no translation for \"{language}\""
            )));
        }
        config.language = Some(language);
    }
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
    if let Some(show_suggestions) = updates.show_suggestions {
        config.show_suggestions = show_suggestions;
    }
    if let Some(markdown_font) = updates.markdown_font {
        config.markdown_font = markdown_font;
    }
    if let Some(markdown_indent) = updates.markdown_indent {
        config.markdown_indent = markdown_indent;
    }
    if let Some(v) = updates.print_margin_top {
        config.print_margin_top = v;
    }
    if let Some(v) = updates.print_margin_bottom {
        config.print_margin_bottom = v;
    }
    if let Some(v) = updates.print_margin_left {
        config.print_margin_left = v;
    }
    if let Some(v) = updates.print_margin_right {
        config.print_margin_right = v;
    }
    if let Some(backend) = updates.backend {
        config.backend = backend;
    }
    if let Some(docker_image) = updates.docker_image {
        config.docker_image = docker_image;
    }
    if let Some(wsl_distro) = updates.wsl_distro {
        config.wsl_distro = wsl_distro;
    }
    if let Some(container_engine) = updates.container_engine {
        config.container_engine = container_engine;
    }
    if let Some(mcp_enabled) = updates.mcp_enabled {
        config.mcp_enabled = mcp_enabled;
    }
    if let Some(mcp_listen_address) = updates.mcp_listen_address {
        config.mcp_listen_address = mcp_listen_address;
    }
    if let Some(mcp_token) = updates.mcp_token {
        config.mcp_token = mcp_token;
    }
    if let Some(section) = updates.settings_section {
        config.settings_section = section;
    }
    write_config(&app, &config)?;

    if config.language() != language_before {
        apply_language(&app, &config.language()).await;
    }

    // Handle MCP server lifecycle changes
    if mcp_changed {
        let state = app.state::<AppState>();
        let controller = state.mcp_controller.clone();

        if controller.is_running().await {
            emit_app_log(
                &state.app_handle,
                &state.app_log,
                "info",
                "MCP server stopping...",
                "mcp",
            );
            controller.stop().await;
        }

        if config.mcp_enabled {
            emit_app_log(
                &state.app_handle,
                &state.app_log,
                "info",
                &format!("MCP server starting on {}...", config.mcp_listen_address),
                "mcp",
            );
            let mcp_state = AppState {
                registry: state.registry.clone(),
                catalog: state.catalog.clone(),
                packages: state.packages.clone(),
                app_handle: state.app_handle.clone(),
                mcp_controller: state.mcp_controller.clone(),
                app_log: state.app_log.clone(),
                initial_file_args: state.initial_file_args.clone(),
            };
            let ct = CancellationToken::new();
            let generation = controller
                .set_running(ct.clone(), &config.mcp_listen_address)
                .await;
            tokio::spawn(crate::mcp::start_mcp_server(
                mcp_state,
                config.mcp_listen_address,
                config.mcp_token,
                ct,
                generation,
            ));
        }
    }

    Ok(())
}

impl AppConfig {
    /// The language in use: the user's choice, or else the system's when
    /// Aximar has a translation for it, or else English
    fn language(&self) -> String {
        self.language.clone().unwrap_or_else(|| {
            i18n::default_language(i18n::system_language().as_deref(), &i18n::languages())
        })
    }

    /// The Maxima a new session runs, and its evaluation timeout.
    fn session_settings(&self) -> SessionSettings {
        SessionSettings {
            backend: Backend::from_config(
                self.backend,
                &self.docker_image,
                &self.wsl_distro,
                self.container_engine,
            ),
            maxima_path: self.maxima_path.clone(),
            eval_timeout: self.eval_timeout,
        }
    }
}

/// The language in use, from the settings file now
pub fn read_language(app: &tauri::AppHandle) -> String {
    read_config(app)
        .map(|(c, _)| c)
        .unwrap_or_default()
        .language()
}

/// Use `language` for the menus and for Maxima's messages in every
/// notebook, from each session's next cell.  The window switches itself.
pub async fn apply_language(app: &tauri::AppHandle, language: &str) {
    let state = app.state::<AppState>();
    let maxima = (language != i18n::ENGLISH).then(|| language.to_string());
    state.registry.lock().await.set_language(maxima);
    i18n::set_help_language(language);
    if let Err(e) = crate::menu::set_menu_language(app, language) {
        tracing::warn!("menu in {language}: {e}");
    }
    // Every window switches, the one that changed the setting included
    let _ = app.emit("language-changed", language);
}

/// The session settings in the settings file now, or the defaults when it
/// can't be read.  Read each time a session starts or a cell runs, so changes
/// in Settings apply without restarting the app.
pub fn read_session_settings(app: &tauri::AppHandle) -> SessionSettings {
    read_config(app)
        .map(|(c, _)| c)
        .unwrap_or_default()
        .session_settings()
}

/// The Maxima the Local backend would run with this Maxima path setting,
/// and where it came from.  Starts nothing.
#[tauri::command]
pub async fn maxima_status(
    maxima_path: Option<String>,
) -> Result<aximar_core::maxima::locate::MaximaLocation, AppError> {
    Ok(aximar_core::maxima::locate::locate_maxima(
        maxima_path.as_deref(),
    ))
}

/// The version a Maxima program reports, such as `5.47.0`.  Runs it once
/// with `--version`, for the Settings window.
#[tauri::command]
pub async fn maxima_version(path: String) -> Result<Option<String>, AppError> {
    Ok(aximar_core::maxima::locate::maxima_version(std::path::Path::new(&path)).await)
}

#[tauri::command]
pub async fn list_wsl_distros() -> Result<Vec<String>, AppError> {
    let mut cmd = tokio::process::Command::new("wsl");
    cmd.args(["-l", "-q"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    hide_console_window(&mut cmd);
    let output = cmd
        .output()
        .await
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
    let output = cmd.output().await.map_err(|e| {
        AppError::ProcessStartFailed(format!("Failed to check maxima in WSL: {}", e))
    })?;

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
/// When the file can't be parsed, the defaults are used for this run only and
/// nothing is written.
pub fn read_mcp_config(app: &tauri::AppHandle) -> McpConfig {
    let config = match read_config_for_update(app) {
        Ok(config) => {
            // Persist so that generated defaults (especially mcp_token) are saved to disk.
            let _ = write_config(app, &config);
            config
        }
        Err(_) => AppConfig::default(),
    };
    McpConfig {
        enabled: config.mcp_enabled,
        listen_address: config.mcp_listen_address,
        token: config.mcp_token,
    }
}

/// Drain all buffered app log entries. Called by the frontend on mount to
/// replay any logs emitted before the event listener was ready.
#[tauri::command]
pub async fn get_buffered_logs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AppLogEvent>, AppError> {
    Ok(state.app_log.drain())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path() -> &'static Path {
        Path::new("/home/user/.config/aximar/config.json")
    }

    #[test]
    fn missing_file_gives_defaults() {
        let file = parse_config_file(path(), None);
        assert!(matches!(file, ConfigFile::Missing));
        let (config, warnings) = config_or_defaults(file);
        assert_eq!(config.font_size, default_font_size());
        assert!(warnings.is_empty());
        assert!(config_for_update(parse_config_file(path(), None)).is_ok());
    }

    #[test]
    fn suggestions_show_unless_turned_off() {
        let (config, _) =
            config_or_defaults(parse_config_file(path(), Some(r#"{"font_size": 14}"#)));
        assert!(config.show_suggestions);
        let (config, _) = config_or_defaults(parse_config_file(
            path(),
            Some(r#"{"show_suggestions": false}"#),
        ));
        assert!(!config.show_suggestions);
    }

    #[test]
    fn valid_file_is_used() {
        let text = r#"{"font_size": 20, "mcp_token": "kept"}"#;
        let (config, warnings) = config_or_defaults(parse_config_file(path(), Some(text)));
        assert_eq!(config.font_size, 20);
        assert!(warnings.is_empty());
        let config = config_for_update(parse_config_file(path(), Some(text))).unwrap();
        assert_eq!(config.mcp_token, "kept");
    }

    #[test]
    fn session_settings_come_from_the_file() {
        let text = r#"{"backend": "wsl", "wsl_distro": "Ubuntu", "maxima_path": "/opt/maxima/bin/maxima", "eval_timeout": 120}"#;
        let (config, _) = config_or_defaults(parse_config_file(path(), Some(text)));
        let settings = config.session_settings();
        assert!(matches!(settings.backend, Backend::Wsl { ref distro } if distro == "Ubuntu"));
        assert_eq!(
            settings.maxima_path.as_deref(),
            Some("/opt/maxima/bin/maxima")
        );
        assert_eq!(settings.eval_timeout, 120);
    }

    #[test]
    fn session_settings_default_to_local_maxima() {
        let text = r#"{"eval_timeout": 0,"#;
        let (config, _) = config_or_defaults(parse_config_file(path(), Some(text)));
        let settings = config.session_settings();
        assert!(matches!(settings.backend, Backend::Local));
        assert_eq!(settings.maxima_path, None);
        assert_eq!(settings.eval_timeout, default_eval_timeout());
    }

    #[test]
    fn settings_section_defaults_to_general() {
        assert_eq!(AppConfig::default().settings_section, "general");
    }

    #[test]
    fn an_older_file_without_settings_section_reads() {
        let text = r#"{"font_size": 20}"#;
        let (config, warnings) = config_or_defaults(parse_config_file(path(), Some(text)));
        assert_eq!(config.settings_section, "general");
        assert!(warnings.is_empty());
        let text = r#"{"settings_section": "printing"}"#;
        let (config, _) = config_or_defaults(parse_config_file(path(), Some(text)));
        assert_eq!(config.settings_section, "printing");
    }

    #[test]
    fn invalid_file_gives_defaults_and_a_warning() {
        let text = r#"{"font_size": 20,"#;
        let (config, warnings) = config_or_defaults(parse_config_file(path(), Some(text)));
        assert_eq!(config.font_size, default_font_size());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("config.json"));
    }

    #[test]
    fn invalid_file_refuses_updates() {
        let text = r#"{"font_size": 20,"#;
        match config_for_update(parse_config_file(path(), Some(text))) {
            Err(AppError::InvalidConfig(message)) => assert!(message.contains("config.json")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }
}
