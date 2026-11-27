//! Aximar's MCP server in AI clients' configurations: whether each client
//! is installed and lists the server, and adding the server to it.
//!
//! Finding out runs no client's program.  Aximar looks the commands up on
//! PATH and reads Codex's and Gemini CLI's own files.  Claude Code's CLI is
//! the only supported way to read its configuration, so Aximar runs
//! `claude mcp get aximar`, but only when the user asks (`claude_mcp_check`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use aximar_core::error::AppError;
use aximar_core::maxima::noconsole::hide_console_window;

/// How long `claude mcp get` may take before Aximar gives up on it
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);

/// Whether a client's configuration lists Aximar's server
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum ConfigState {
    Yes,
    No,
    /// Not known until the user asks: Claude Code's, before a check
    Unchecked,
    /// The check took longer than its time limit
    TimedOut,
    /// The client's file can't be read; Aximar leaves it alone
    Error {
        file: String,
        detail: String,
    },
}

/// One client: where its command is, and what its configuration says
#[derive(Debug, Clone, Serialize)]
pub struct ClientStatus {
    /// The command's path, or `None` if it isn't installed
    pub installed: Option<String>,
    pub configured: ConfigState,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpClientsStatus {
    pub claude: ClientStatus,
    pub codex: ClientStatus,
    pub gemini: ClientStatus,
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// Where these CLIs install besides PATH.  An app opened from the macOS
/// Finder gets a short PATH, without the user's shell additions.
fn extra_dirs(home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = home {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".claude/local"));
        dirs.push(home.join(".npm-global/bin"));
    }
    if cfg!(unix) {
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
    }
    dirs
}

/// The directories to search: PATH's, then the extra ones
fn search_path(path: Option<OsString>, extra: &[PathBuf]) -> OsString {
    let mut dirs: Vec<PathBuf> = path
        .as_deref()
        .map(|p| std::env::split_paths(p).collect())
        .unwrap_or_default();
    dirs.extend(extra.iter().cloned());
    std::env::join_paths(dirs).unwrap_or_default()
}

/// The executable file a command name runs, from `search`, without
/// running it
fn find_command_in(name: &str, search: &OsString) -> Option<PathBuf> {
    let cwd = std::env::temp_dir();
    which::which_in(name, Some(search), cwd).ok()
}

/// Where a client's command is installed, if it is
pub fn find_command(name: &str) -> Option<PathBuf> {
    let home = home_dir();
    let search = search_path(std::env::var_os("PATH"), &extra_dirs(home.as_deref()));
    find_command_in(name, &search)
}

fn read_if_present(path: &Path) -> Result<Option<String>, ConfigState> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(ConfigState::Error {
            file: path.display().to_string(),
            detail: e.to_string(),
        }),
    }
}

/// Whether `~/.codex/config.toml` lists Aximar under `mcp_servers`
fn codex_state(home: &Path) -> ConfigState {
    let path = home.join(".codex").join("config.toml");
    let text = match read_if_present(&path) {
        Ok(Some(text)) => text,
        Ok(None) => return ConfigState::No,
        Err(state) => return state,
    };
    match text.parse::<toml_edit::DocumentMut>() {
        Ok(doc) => {
            let listed = doc
                .get("mcp_servers")
                .and_then(|s| s.as_table_like())
                .and_then(|s| s.get("aximar"))
                .is_some();
            if listed {
                ConfigState::Yes
            } else {
                ConfigState::No
            }
        }
        Err(e) => ConfigState::Error {
            file: path.display().to_string(),
            detail: e.to_string(),
        },
    }
}

/// Whether `~/.gemini/settings.json` lists Aximar under `mcpServers`
fn gemini_state(home: &Path) -> ConfigState {
    let path = home.join(".gemini").join("settings.json");
    let text = match read_if_present(&path) {
        Ok(Some(text)) => text,
        Ok(None) => return ConfigState::No,
        Err(state) => return state,
    };
    if text.trim().is_empty() {
        return ConfigState::No;
    }
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(json) => {
            if json
                .get("mcpServers")
                .and_then(|s| s.get("aximar"))
                .is_some()
            {
                ConfigState::Yes
            } else {
                ConfigState::No
            }
        }
        Err(e) => ConfigState::Error {
            file: path.display().to_string(),
            detail: e.to_string(),
        },
    }
}

fn clients_status(home: Option<&Path>, find: impl Fn(&str) -> Option<PathBuf>) -> McpClientsStatus {
    let installed = |name: &str| find(name).map(|p| p.display().to_string());
    let from_file = |state: fn(&Path) -> ConfigState| match home {
        Some(home) => state(home),
        None => ConfigState::No,
    };
    McpClientsStatus {
        claude: ClientStatus {
            installed: installed("claude"),
            configured: ConfigState::Unchecked,
        },
        codex: ClientStatus {
            installed: installed("codex"),
            configured: from_file(codex_state),
        },
        gemini: ClientStatus {
            installed: installed("gemini"),
            configured: from_file(gemini_state),
        },
    }
}

/// Every client's status, without running any of them
#[tauri::command]
pub async fn mcp_clients_status() -> Result<McpClientsStatus, AppError> {
    Ok(clients_status(home_dir().as_deref(), find_command))
}

/// Run `<claude> mcp get aximar`, giving up after `timeout`
async fn run_check(program: &Path, timeout: Duration) -> ConfigState {
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(["mcp", "get", "aximar"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    hide_console_window(&mut cmd);
    match tokio::time::timeout(timeout, cmd.output()).await {
        Err(_) => ConfigState::TimedOut,
        Ok(Err(e)) => ConfigState::Error {
            file: program.display().to_string(),
            detail: e.to_string(),
        },
        Ok(Ok(output)) if output.status.success() => ConfigState::Yes,
        Ok(Ok(_)) => ConfigState::No,
    }
}

fn claude_not_found() -> AppError {
    AppError::ProcessStartFailed("The claude command isn't installed".into())
}

/// Ask Claude Code whether it lists Aximar's server.  Runs its CLI, so the
/// window calls this only when the user presses Check, or after Configure.
#[tauri::command]
pub async fn claude_mcp_check() -> Result<ConfigState, AppError> {
    let claude = find_command("claude").ok_or_else(claude_not_found)?;
    Ok(run_check(&claude, CHECK_TIMEOUT).await)
}

#[tauri::command]
pub async fn claude_mcp_configure(url: String, token: String) -> Result<String, AppError> {
    let claude = find_command("claude").ok_or_else(claude_not_found)?;

    // Remove any existing entry; it may not exist
    let mut rm_cmd = tokio::process::Command::new(&claude);
    rm_cmd
        .args(["mcp", "remove", "aximar"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    hide_console_window(&mut rm_cmd);
    let _ = rm_cmd.output().await;

    let header = format!("Authorization: Bearer {token}");
    let mut add_cmd = tokio::process::Command::new(&claude);
    add_cmd
        .args([
            "mcp",
            "add",
            "--transport",
            "http",
            "--header",
            &header,
            "--",
            "aximar",
            &url,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    hide_console_window(&mut add_cmd);

    let output = add_cmd
        .output()
        .await
        .map_err(|e| AppError::ProcessStartFailed(format!("Failed to run claude CLI: {e}")))?;

    if output.status.success() {
        Ok("Claude Code MCP configured successfully".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(AppError::ProcessStartFailed(format!(
            "claude mcp add failed: {stderr}"
        )))
    }
}

#[tauri::command]
pub async fn codex_mcp_configure(url: String, token: String) -> Result<String, AppError> {
    use toml_edit::{value, DocumentMut, InlineTable};

    let home = home_dir()
        .ok_or_else(|| AppError::ProcessStartFailed("Could not determine home directory".into()))?;
    let config_dir = home.join(".codex");
    let config_path = config_dir.join("config.toml");

    // Read and parse the existing config, or start with an empty document
    let content = if config_path.exists() {
        tokio::fs::read_to_string(&config_path).await?
    } else {
        String::new()
    };
    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| AppError::ProcessStartFailed(format!("Failed to parse config.toml: {e}")))?;

    if !doc.contains_table("mcp_servers") {
        doc["mcp_servers"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    // The [mcp_servers.aximar] entry
    let servers = doc["mcp_servers"]
        .as_table_mut()
        .expect("mcp_servers is a table");
    let mut entry = toml_edit::Table::new();
    entry["url"] = value(&url);
    let mut headers = InlineTable::new();
    headers.insert("Authorization", format!("Bearer {token}").into());
    entry["http_headers"] = toml_edit::Item::Value(toml_edit::Value::InlineTable(headers));
    servers["aximar"] = toml_edit::Item::Table(entry);

    tokio::fs::create_dir_all(&config_dir).await.ok();
    tokio::fs::write(&config_path, doc.to_string())
        .await
        .map_err(|e| AppError::ProcessStartFailed(format!("Failed to write config.toml: {e}")))?;

    Ok("Codex MCP configured successfully".to_string())
}

#[tauri::command]
pub async fn gemini_mcp_configure(url: String, token: String) -> Result<String, AppError> {
    let home = home_dir()
        .ok_or_else(|| AppError::ProcessStartFailed("Could not determine home directory".into()))?;
    let config_dir = home.join(".gemini");
    let settings_path = config_dir.join("settings.json");

    // Read existing settings or start with an empty object.  A file that
    // can't be read or parsed is left alone: writing back only our entry
    // would discard the user's other Gemini settings.
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = tokio::fs::read_to_string(&settings_path).await?;
        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&content).map_err(|e| {
                AppError::InvalidConfig(format!(
                    "{} can't be read ({e}). Fix or remove it, then try again.",
                    settings_path.display()
                ))
            })?
        }
    } else {
        serde_json::json!({})
    };

    if settings.get("mcpServers").is_none() {
        settings["mcpServers"] = serde_json::json!({});
    }

    settings["mcpServers"]["aximar"] = serde_json::json!({
        "httpUrl": url,
        "headers": {
            "Authorization": format!("Bearer {token}")
        }
    });

    let output = serde_json::to_string_pretty(&settings)
        .map_err(|e| AppError::ProcessStartFailed(format!("Failed to serialize settings: {e}")))?;

    tokio::fs::create_dir_all(&config_dir).await.ok();
    tokio::fs::write(&settings_path, output)
        .await
        .map_err(|e| AppError::ProcessStartFailed(format!("Failed to write settings.json: {e}")))?;

    Ok("Gemini CLI MCP configured successfully".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An executable file named `name` in `dir`
    fn executable(dir: &Path, name: &str, body: &str) -> PathBuf {
        let file = if cfg!(windows) {
            dir.join(format!("{name}.bat"))
        } else {
            dir.join(name)
        };
        std::fs::write(&file, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        file
    }

    #[test]
    fn finds_a_command_on_path() {
        let dir = tempfile::tempdir().unwrap();
        let file = executable(dir.path(), "aximar-test-cli", "#!/bin/sh\n");
        let search = search_path(Some(dir.path().as_os_str().to_owned()), &[]);
        assert_eq!(find_command_in("aximar-test-cli", &search), Some(file));
    }

    #[test]
    fn finds_a_command_in_an_extra_directory() {
        let path = tempfile::tempdir().unwrap();
        let extra = tempfile::tempdir().unwrap();
        let file = executable(extra.path(), "aximar-test-cli", "#!/bin/sh\n");
        let search = search_path(
            Some(path.path().as_os_str().to_owned()),
            &[extra.path().to_path_buf()],
        );
        assert_eq!(find_command_in("aximar-test-cli", &search), Some(file));
    }

    #[test]
    fn a_missing_command_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let search = search_path(Some(dir.path().as_os_str().to_owned()), &[]);
        assert_eq!(find_command_in("aximar-test-cli", &search), None);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_that_is_not_executable_is_not_a_command() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("aximar-test-cli"), "").unwrap();
        let search = search_path(Some(dir.path().as_os_str().to_owned()), &[]);
        assert_eq!(find_command_in("aximar-test-cli", &search), None);
    }

    #[test]
    fn codex_configured_from_its_file() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join(".codex")).unwrap();
        std::fs::write(
            home.path().join(".codex/config.toml"),
            "model = \"o3\"\n\n[mcp_servers.aximar]\nurl = \"http://127.0.0.1:19542/mcp\"\n",
        )
        .unwrap();
        assert_eq!(codex_state(home.path()), ConfigState::Yes);
    }

    #[test]
    fn codex_without_aximar_is_not_configured() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join(".codex")).unwrap();
        std::fs::write(
            home.path().join(".codex/config.toml"),
            "[mcp_servers.other]\nurl = \"http://x\"\n",
        )
        .unwrap();
        assert_eq!(codex_state(home.path()), ConfigState::No);
    }

    #[test]
    fn codex_without_a_file_is_not_configured() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(codex_state(home.path()), ConfigState::No);
    }

    #[test]
    fn a_broken_gemini_file_is_an_error() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join(".gemini")).unwrap();
        std::fs::write(home.path().join(".gemini/settings.json"), "{ not json").unwrap();
        match gemini_state(home.path()) {
            ConfigState::Error { file, .. } => assert!(file.ends_with("settings.json"), "{file}"),
            other => panic!("expected an error, got {other:?}"),
        }
    }

    #[test]
    fn gemini_configured_from_its_file() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir(home.path().join(".gemini")).unwrap();
        std::fs::write(
            home.path().join(".gemini/settings.json"),
            r#"{"theme": "dark", "mcpServers": {"aximar": {"httpUrl": "http://x"}}}"#,
        )
        .unwrap();
        assert_eq!(gemini_state(home.path()), ConfigState::Yes);
    }

    #[test]
    fn claude_is_unchecked() {
        let home = tempfile::tempdir().unwrap();
        let status = clients_status(Some(home.path()), |name| {
            (name == "claude").then(|| PathBuf::from("/bin/claude"))
        });
        assert_eq!(status.claude.installed.as_deref(), Some("/bin/claude"));
        assert_eq!(status.claude.configured, ConfigState::Unchecked);
        assert_eq!(status.codex.installed, None);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_slow_check_times_out() {
        let dir = tempfile::tempdir().unwrap();
        let stub = executable(dir.path(), "claude", "#!/bin/sh\nsleep 20\n");
        let started = std::time::Instant::now();
        let state = run_check(&stub, Duration::from_millis(300)).await;
        assert_eq!(state, ConfigState::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_check_reads_the_exit_status() {
        let dir = tempfile::tempdir().unwrap();
        let yes = executable(dir.path(), "claude", "#!/bin/sh\nexit 0\n");
        assert_eq!(run_check(&yes, CHECK_TIMEOUT).await, ConfigState::Yes);
        let no = executable(dir.path(), "claude-no", "#!/bin/sh\nexit 1\n");
        assert_eq!(run_check(&no, CHECK_TIMEOUT).await, ConfigState::No);
    }
}
