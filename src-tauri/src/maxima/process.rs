use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::error::AppError;
use crate::maxima::backend::Backend;
use crate::maxima::noconsole::hide_console_window;

const READY_SENTINEL: &str = "__AXIMAR_READY__";

#[derive(Clone, Serialize)]
pub struct MaximaOutputEvent {
    pub line: String,
    pub stream: String,
    pub timestamp: u64,
}

pub struct MaximaProcess {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_reader: BufReader<tokio::process::ChildStdout>,
    stderr_reader: BufReader<tokio::process::ChildStderr>,
    backend: Backend,
    container_name: Option<String>,
    app_handle: Option<tauri::AppHandle>,
}

impl MaximaProcess {
    pub async fn spawn(backend: Backend, custom_path: Option<String>, app_handle: Option<tauri::AppHandle>) -> Result<Self, AppError> {
        Self::preflight_check(&backend).await?;

        let (mut child, container_name) = match &backend {
            Backend::Local => {
                let maxima_path = custom_path
                    .filter(|p| !p.is_empty())
                    .unwrap_or_else(find_maxima_binary);

                let mut cmd = Command::new(&maxima_path);
                cmd.arg("--very-quiet")
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true);
                hide_console_window(&mut cmd);
                let child = cmd.spawn()
                    .map_err(|e| AppError::ProcessStartFailed(format!("{}: {}", maxima_path, e)))?;

                (child, None)
            }
            Backend::Docker { engine, image } => {
                if image.is_empty() {
                    return Err(AppError::ProcessStartFailed(
                        "Docker image not configured. Set a Docker image in Settings.".into(),
                    ));
                }

                let container_name = format!("aximar-{}", std::process::id());

                // Ensure host temp dir exists
                if let Some(host_dir) = backend.host_temp_dir() {
                    std::fs::create_dir_all(&host_dir).map_err(|e| {
                        AppError::ProcessStartFailed(format!(
                            "Failed to create temp directory {}: {}",
                            host_dir.display(),
                            e
                        ))
                    })?;
                }

                let volume_mount = format!(
                    "{}:{}",
                    backend
                        .host_temp_dir()
                        .unwrap()
                        .to_string_lossy(),
                    Backend::container_temp_dir()
                );

                // GCL (the Lisp runtime Ubuntu's Maxima uses) calls
                // personality(ADDR_NO_RANDOMIZE | READ_IMPLIES_EXEC) which
                // Docker's default seccomp profile blocks. We use a custom
                // profile that adds only these specific personality values
                // rather than disabling seccomp entirely.
                let seccomp_path = Backend::write_seccomp_profile().map_err(|e| {
                    AppError::ProcessStartFailed(format!(
                        "Failed to write seccomp profile: {}", e
                    ))
                })?;
                let seccomp_opt = format!("seccomp={}", seccomp_path.display());

                let mut docker_cmd = Command::new(engine);
                docker_cmd.args([
                        "run",
                        "--rm",
                        "-i",
                        "--network",
                        "none",
                        "--memory",
                        "512m",
                        "--security-opt",
                        &seccomp_opt,
                        "--name",
                        &container_name,
                        "-v",
                        &volume_mount,
                        image,
                        "--very-quiet",
                    ])
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true);
                hide_console_window(&mut docker_cmd);
                let child = docker_cmd.spawn()
                    .map_err(|e| {
                        AppError::ProcessStartFailed(format!("{} run {}: {}", engine, image, e))
                    })?;

                (child, Some(container_name))
            }
            Backend::Wsl { distro } => {
                // Ensure host temp dir exists for plot SVG copies
                if let Some(host_dir) = backend.host_temp_dir() {
                    std::fs::create_dir_all(&host_dir).map_err(|e| {
                        AppError::ProcessStartFailed(format!(
                            "Failed to create temp directory {}: {}",
                            host_dir.display(),
                            e
                        ))
                    })?;
                }

                // Create the temp dir inside WSL for gnuplot output
                let mut mkdir_cmd = Command::new("wsl");
                if !distro.is_empty() {
                    mkdir_cmd.args(["-d", distro]);
                }
                mkdir_cmd.args(["--", "mkdir", "-p", Backend::container_temp_dir()]);
                mkdir_cmd
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                hide_console_window(&mut mkdir_cmd);
                let _ = mkdir_cmd.status().await;

                let mut cmd = Command::new("wsl");
                if !distro.is_empty() {
                    cmd.args(["-d", distro]);
                }
                cmd.args(["--", "maxima", "--very-quiet"]);
                cmd.stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true);
                hide_console_window(&mut cmd);

                let child = cmd.spawn().map_err(|e| {
                    AppError::ProcessStartFailed(format!("wsl maxima: {}", e))
                })?;

                (child, None)
            }
        };

        let stdin = child.stdin.take().ok_or_else(|| {
            AppError::ProcessStartFailed("Failed to capture stdin".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            AppError::ProcessStartFailed("Failed to capture stdout".into())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            AppError::ProcessStartFailed("Failed to capture stderr".into())
        })?;
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let mut proc = MaximaProcess {
            child,
            stdin,
            stdout_reader,
            stderr_reader,
            backend,
            container_name,
            app_handle,
        };

        proc.initialize().await?;
        Ok(proc)
    }

    async fn initialize(&mut self) -> Result<(), AppError> {
        // For Docker/WSL, set maxima_tempdir so gnuplot writes SVGs to a known location
        let tempdir_cmd = match &self.backend {
            Backend::Docker { .. } | Backend::Wsl { .. } => {
                format!("maxima_tempdir:\"{}\"$\n", Backend::container_temp_dir())
            }
            _ => String::new(),
        };

        let init_commands = format!(
            "{}display2d:false$\nset_plot_option([run_viewer, false])$\nset_plot_option([gnuplot_term, svg])$\nprint(\"{}\")$\n",
            tempdir_cmd,
            READY_SENTINEL
        );

        self.write_stdin(&init_commands).await?;
        self.read_until_sentinel(READY_SENTINEL).await?;

        // Set % to a harmless value so tex(%) after an error in the first cell
        // doesn't render the sentinel string
        self.write_stdin("0$\n").await?;

        Ok(())
    }

    fn emit_output(&self, line: &str, stream: &str) {
        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit("maxima-output", MaximaOutputEvent {
                line: line.to_string(),
                stream: stream.to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            });
        }
    }

    pub async fn write_stdin(&mut self, input: &str) -> Result<(), AppError> {
        for line in input.lines() {
            if !line.is_empty() {
                self.emit_output(line, "stdin");
            }
        }
        self.stdin
            .write_all(input.as_bytes())
            .await
            .map_err(|e| AppError::CommunicationError(e.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| AppError::CommunicationError(e.to_string()))?;
        Ok(())
    }

    pub async fn read_until_sentinel(&mut self, sentinel: &str) -> Result<Vec<String>, AppError> {
        let mut lines = Vec::new();
        let mut stdout_line = String::new();
        let mut stderr_line = String::new();

        loop {
            stdout_line.clear();
            stderr_line.clear();

            // Read from both stdout and stderr concurrently
            tokio::select! {
                result = self.stdout_reader.read_line(&mut stdout_line) => {
                    let bytes_read = result.map_err(|e| AppError::CommunicationError(e.to_string()))?;
                    if bytes_read == 0 {
                        return Err(AppError::CommunicationError(
                            "Maxima process closed unexpectedly".into(),
                        ));
                    }
                    let trimmed = stdout_line.trim_end().to_string();
                    self.emit_output(&trimmed, "stdout");
                    if trimmed.contains(sentinel) {
                        lines.push(trimmed);
                        // Drain any remaining stderr
                        self.drain_stderr(&mut lines).await;
                        // Read one more stdout line (the print() return value)
                        let mut extra = String::new();
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_millis(200),
                            self.stdout_reader.read_line(&mut extra),
                        ).await;
                        return Ok(lines);
                    }
                    lines.push(trimmed);
                }
                result = self.stderr_reader.read_line(&mut stderr_line) => {
                    let bytes_read = result.map_err(|e| AppError::CommunicationError(e.to_string()))?;
                    if bytes_read == 0 {
                        // stderr closed, not fatal — continue reading stdout
                        continue;
                    }
                    let trimmed = stderr_line.trim_end().to_string();
                    if !trimmed.is_empty() {
                        self.emit_output(&trimmed, "stderr");
                        lines.push(trimmed);
                    }
                }
            };
        }
    }

    async fn drain_stderr(&mut self, lines: &mut Vec<String>) {
        let mut line = String::new();
        loop {
            line.clear();
            match tokio::time::timeout(
                std::time::Duration::from_millis(50),
                self.stderr_reader.read_line(&mut line),
            ).await {
                Ok(Ok(n)) if n > 0 => {
                    let trimmed = line.trim_end().to_string();
                    if !trimmed.is_empty() {
                        self.emit_output(&trimmed, "stderr");
                        lines.push(trimmed);
                    }
                }
                _ => break,
            }
        }
    }

    pub async fn kill(&mut self) -> Result<(), AppError> {
        self.child.kill().await.map_err(AppError::Io)?;

        // For Docker/Podman, also force-remove the container as a safety net
        if let (Backend::Docker { engine, .. }, Some(name)) =
            (&self.backend, &self.container_name)
        {
            let mut rm_cmd = tokio::process::Command::new(engine);
            rm_cmd.args(["rm", "-f", name]);
            hide_console_window(&mut rm_cmd);
            let _ = rm_cmd.output().await;
        }

        Ok(())
    }

    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    async fn preflight_check(backend: &Backend) -> Result<(), AppError> {
        match backend {
            Backend::Local => Ok(()),
            Backend::Docker { engine, image } => {
                // Check engine is available
                let mut cmd = tokio::process::Command::new(engine);
                cmd.arg("info")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::piped());
                hide_console_window(&mut cmd);
                let output = cmd.output().await
                    .map_err(|e| {
                        AppError::ProcessStartFailed(format!(
                            "'{}' not found. Is {} installed and running? {}",
                            engine, engine, e
                        ))
                    })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(AppError::ProcessStartFailed(format!(
                        "{} daemon not responding: {}",
                        engine,
                        stderr.trim()
                    )));
                }

                // Check image exists
                if !image.is_empty() {
                    let mut cmd = tokio::process::Command::new(engine);
                    cmd.args(["image", "inspect", image])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::piped());
                    hide_console_window(&mut cmd);
                    let output = cmd.output().await
                        .map_err(|e| {
                            AppError::ProcessStartFailed(format!(
                                "Failed to check image '{}': {}",
                                image, e
                            ))
                        })?;

                    if !output.status.success() {
                        return Err(AppError::ProcessStartFailed(format!(
                            "Image '{}' not found. Pull it with: {} pull {}",
                            image, engine, image
                        )));
                    }
                }

                Ok(())
            }
            Backend::Wsl { distro } => {
                // Check WSL is available
                let mut cmd = tokio::process::Command::new("wsl");
                cmd.arg("--status")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::piped());
                hide_console_window(&mut cmd);
                let output = cmd.output().await
                    .map_err(|e| {
                        AppError::ProcessStartFailed(format!(
                            "'wsl' not found. Is WSL installed? {}",
                            e
                        ))
                    })?;

                if !output.status.success() {
                    return Err(AppError::ProcessStartFailed(
                        "WSL is not available or not running.".into(),
                    ));
                }

                // Check distro exists if specified
                if !distro.is_empty() {
                    let mut cmd = tokio::process::Command::new("wsl");
                    cmd.args(["-d", distro, "--", "echo", "ok"])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::piped());
                    hide_console_window(&mut cmd);
                    let output = cmd.output().await
                        .map_err(|e| {
                            AppError::ProcessStartFailed(format!(
                                "Failed to check WSL distro '{}': {}",
                                distro, e
                            ))
                        })?;

                    if !output.status.success() {
                        return Err(AppError::ProcessStartFailed(format!(
                            "WSL distribution '{}' not found.",
                            distro
                        )));
                    }
                }

                // Check maxima is installed in the WSL distro
                {
                    let mut cmd = tokio::process::Command::new("wsl");
                    if !distro.is_empty() {
                        cmd.args(["-d", distro]);
                    }
                    cmd.args(["--", "which", "maxima"]);
                    cmd.stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::piped());
                    hide_console_window(&mut cmd);
                    let output = cmd.output().await
                        .map_err(|e| {
                            AppError::ProcessStartFailed(format!(
                                "Failed to check for maxima in WSL: {}",
                                e
                            ))
                        })?;

                    if !output.status.success() {
                        let distro_hint = if distro.is_empty() {
                            "the default WSL distribution".to_string()
                        } else {
                            format!("WSL distribution '{}'", distro)
                        };
                        return Err(AppError::ProcessStartFailed(format!(
                            "'maxima' not found in {}. Install it with: sudo apt install maxima",
                            distro_hint
                        )));
                    }
                }

                Ok(())
            }
        }
    }
}

fn find_maxima_binary() -> String {
    if let Ok(path) = env::var("AXIMAR_MAXIMA_PATH") {
        return path;
    }

    let candidates: Vec<PathBuf> = if cfg!(target_os = "windows") {
        let mut paths = Vec::new();
        if let Ok(entries) = std::fs::read_dir("C:\\") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("maxima-") {
                    paths.push(entry.path().join("bin").join("maxima.bat"));
                }
            }
        }
        paths
    } else {
        vec![
            PathBuf::from("/opt/homebrew/bin/maxima"),
            PathBuf::from("/usr/local/bin/maxima"),
            PathBuf::from("/usr/bin/maxima"),
        ]
    };

    for candidate in candidates {
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
    }

    "maxima".to_string()
}
