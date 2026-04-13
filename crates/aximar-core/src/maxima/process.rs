use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::error::AppError;
use crate::maxima::backend::Backend;
use crate::maxima::debugger::{self, PromptKind};
use crate::maxima::noconsole::hide_console_window;
use crate::maxima::output::{OutputEvent, OutputSink};

const READY_SENTINEL: &str = "__AXIMAR_READY__";

pub struct MaximaProcess {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_reader: BufReader<tokio::process::ChildStdout>,
    stderr_reader: BufReader<tokio::process::ChildStderr>,
    backend: Backend,
    container_name: Option<String>,
    output_sink: Arc<dyn OutputSink>,
    /// When true, `read_until_sentinel` uses chunk-based reading and
    /// also detects debugger prompts (`dbm:N>`). When false, uses
    /// simpler line-based reading (sentinel only).
    debug_mode: bool,
}

impl MaximaProcess {
    pub async fn spawn(backend: Backend, custom_path: Option<String>, output_sink: Arc<dyn OutputSink>) -> Result<Self, AppError> {
        Self::spawn_with_cwd(backend, custom_path, output_sink, None).await
    }

    pub async fn spawn_with_cwd(backend: Backend, custom_path: Option<String>, output_sink: Arc<dyn OutputSink>, cwd: Option<&std::path::Path>) -> Result<Self, AppError> {
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
                if let Some(dir) = cwd {
                    cmd.current_dir(dir);
                }
                #[cfg(windows)]
                {
                    cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
                }
                // Detach from the controlling terminal so SBCL cannot
                // open /dev/tty.  Without this, *debug-io* on Linux
                // writes to /dev/tty (bypassing our piped stdout/stderr)
                // and we never see debugger prompts or some error messages.
                // On macOS /dev/tty typically already fails to open; on
                // Windows SBCL never tries.
                #[cfg(unix)]
                {
                    // SAFETY: setsid() is async-signal-safe and has no
                    // preconditions beyond being called in the child
                    // process before exec, which pre_exec guarantees.
                    unsafe {
                        cmd.pre_exec(|| {
                            libc::setsid();
                            Ok(())
                        });
                    }
                }
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
                        "-e", "LANG=en_US.UTF-8",
                        "-e", "LC_ALL=en_US.UTF-8",
                        "-v",
                        &volume_mount,
                        image,
                        "--very-quiet",
                    ])
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true);
                #[cfg(windows)]
                {
                    docker_cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
                }
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
                cmd.args(["--", "env", "LANG=en_US.UTF-8", "LC_ALL=en_US.UTF-8", "maxima", "--very-quiet"]);
                cmd.stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true);
                #[cfg(windows)]
                {
                    cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
                }
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
            output_sink,
            debug_mode: false,
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
            "{}display2d:false$\nset_plot_option([run_viewer, false])$\nset_plot_option([gnuplot_term, svg])$\nset_plot_option([gnuplot_preamble, \"set encoding utf8\"])$\nprint(\"{}\")$\n",
            tempdir_cmd,
            READY_SENTINEL
        );

        self.write_stdin(&init_commands).await?;
        self.read_until_sentinel(READY_SENTINEL).await.map(|_| ())?;

        // Set % to a harmless value so tex(%) after an error in the first cell
        // doesn't render the sentinel string
        self.write_stdin("0$\n").await?;

        Ok(())
    }

    /// Enable debug mode. In debug mode, `read_until_sentinel` uses
    /// chunk-based reading and can detect debugger prompts in addition
    /// to sentinels. Call this after `debugmode(true)$` in Maxima.
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }

    fn emit_output(&self, line: &str, stream: &str) {
        self.output_sink.emit(OutputEvent {
            line: line.to_string(),
            stream: stream.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        });
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

    /// Read stdout/stderr until a sentinel string is found (and, in debug
    /// mode, also until a debugger prompt `(dbm:N)` is encountered).
    ///
    /// Returns the collected lines and a [`PromptKind`] indicating what
    /// terminated the read:
    /// - `PromptKind::Normal` — the sentinel was found (normal completion)
    /// - `PromptKind::Debugger { level }` — a debugger prompt appeared
    ///   (debug mode only; never returned in normal mode)
    ///
    /// In **normal mode** (default), uses line-based reading optimized for
    /// MCP/Tauri — debugger prompts are never expected, so simpler I/O suffices.
    ///
    /// In **debug mode** (set via [`set_debug_mode`]), uses chunk-based reading
    /// that can detect debugger prompts even without a trailing newline.
    pub async fn read_until_sentinel(
        &mut self,
        sentinel: &str,
    ) -> Result<(Vec<String>, PromptKind), AppError> {
        if self.debug_mode {
            self.read_dap_response(Some(sentinel)).await
        } else {
            let lines = self.read_until_sentinel_line_based(sentinel).await?;
            Ok((lines, PromptKind::Normal))
        }
    }

    /// Line-based sentinel reader for non-debug mode (MCP/Tauri).
    async fn read_until_sentinel_line_based(
        &mut self,
        sentinel: &str,
    ) -> Result<Vec<String>, AppError> {
        let mut lines = Vec::new();
        let mut stdout_line = String::new();
        let mut stderr_line = String::new();

        loop {
            stdout_line.clear();
            stderr_line.clear();

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

    /// Extract the error message from collected output lines.
    ///
    /// Scans backwards for an error marker line (e.g. `" -- an error."`)
    /// and includes the preceding line(s) which typically contain the
    /// actual error description (e.g. `"ev: improper argument: 601"`).
    fn extract_error_context(lines: &[String]) -> Option<String> {
        for (i, line) in lines.iter().enumerate().rev() {
            if debugger::ERROR_MARKERS.iter().any(|m| line.contains(m)) {
                // Include the line before the error marker if available —
                // that's typically the actual error message.
                let start = if i > 0 { i - 1 } else { i };
                let context: Vec<&str> = lines[start..=i]
                    .iter()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !context.is_empty() {
                    return Some(context.join("\n"));
                }
            }
        }
        None
    }

    /// Send an interrupt signal to the Maxima process.
    /// Unix: SIGINT. Windows: CTRL_BREAK_EVENT (requires CREATE_NEW_PROCESS_GROUP at spawn).
    pub fn interrupt(&self) {
        let Some(pid) = self.child.id() else { return };

        #[cfg(unix)]
        unsafe {
            libc::kill(pid as i32, libc::SIGINT);
        }

        #[cfg(windows)]
        {
            extern "system" {
                fn GenerateConsoleCtrlEvent(dwCtrlEvent: u32, dwProcessGroupId: u32) -> i32;
            }
            const CTRL_BREAK_EVENT: u32 = 1;
            unsafe {
                GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid);
            }
        }
    }

    /// Interrupt a running Maxima computation and drain output to
    /// re-synchronize after a timeout.
    pub async fn interrupt_and_resync(&mut self, sentinel: &str) {
        self.interrupt();

        // Drain output until the sentinel arrives. The sentinel print
        // was already written to stdin before the timeout, so after the
        // interrupt Maxima should process it shortly.
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.read_until_sentinel(sentinel),
        )
        .await;
        // If drain also times out, the process is truly stuck.
        // Caller returns the timeout error; user can restart.
    }

    /// Read Maxima output using chunk-based reading.
    ///
    /// Unlike [`read_until_sentinel_line_based`] which uses
    /// `read_line`, this uses `read()` to detect debugger prompts
    /// (`dbm:N>`) that are written without a trailing newline.
    ///
    /// If `sentinel` is provided, also checks complete lines for the
    /// sentinel and returns [`PromptKind::Normal`] when found.
    /// Returns [`PromptKind::Debugger`] when a debugger prompt is detected.
    ///
    /// Also detects error conditions that would otherwise cause a hang:
    /// - SBCL Lisp debugger prompt (`0]`) — Lisp-level crash
    /// - Maxima error markers (`" -- an error."`) followed by silence —
    ///   parse errors that bypass `debugmode(true)` and leave Maxima
    ///   waiting at an invisible prompt (with `--very-quiet`)
    pub async fn read_dap_response(
        &mut self,
        sentinel: Option<&str>,
    ) -> Result<(Vec<String>, PromptKind), AppError> {
        let mut lines = Vec::new();
        let mut partial = String::new();
        let mut read_buf = [0u8; 4096];
        let mut stderr_line = String::new();

        // After seeing a Maxima error marker, we give debugmode 2 seconds
        // to produce a (dbm:N) prompt.  If it doesn't, the error was a
        // parse error (or similar) that bypassed the debugger and Maxima
        // is silently waiting for input — we'll never get a sentinel.
        let error_grace = std::time::Duration::from_secs(2);
        let mut error_deadline = std::pin::pin!(tokio::time::sleep(error_grace));
        let mut saw_error = false;

        loop {
            stderr_line.clear();

            tokio::select! {
                result = self.stdout_reader.read(&mut read_buf) => {
                    let n = result.map_err(|e| AppError::CommunicationError(e.to_string()))?;
                    if n == 0 {
                        return Err(AppError::CommunicationError(
                            "Maxima process closed unexpectedly".into(),
                        ));
                    }

                    let chunk = String::from_utf8_lossy(&read_buf[..n]);
                    partial.push_str(&chunk);

                    // Extract and process complete lines
                    while let Some(newline_pos) = partial.find('\n') {
                        let line = partial[..newline_pos].trim_end_matches('\r').to_string();
                        partial = partial[newline_pos + 1..].to_string();

                        self.emit_output(&line, "stdout");

                        if let Some(level) = debugger::detect_debugger_prompt(&line) {
                            lines.push(line);
                            self.drain_stderr(&mut lines).await;
                            let error_context = if saw_error {
                                Self::extract_error_context(&lines)
                            } else {
                                None
                            };
                            return Ok((lines, PromptKind::Debugger { level, error_context }));
                        }

                        if let Some(s) = sentinel {
                            if line.contains(s) {
                                lines.push(line);
                                self.drain_stderr(&mut lines).await;
                                return Ok((lines, PromptKind::Normal));
                            }
                        }

                        // Detect Maxima error markers.  Don't return yet —
                        // debugmode may still produce a (dbm:N) prompt.
                        if !saw_error
                            && debugger::ERROR_MARKERS
                                .iter()
                                .any(|m| line.contains(m))
                        {
                            saw_error = true;
                            error_deadline
                                .as_mut()
                                .reset(tokio::time::Instant::now() + error_grace);
                        }

                        lines.push(line);
                    }

                    // Check remaining partial buffer for prompts without
                    // a trailing newline.
                    let trimmed = partial.trim();
                    if !trimmed.is_empty() {
                        if let Some(level) = debugger::detect_debugger_prompt(trimmed) {
                            self.emit_output(trimmed, "stdout");
                            lines.push(trimmed.to_string());
                            partial.clear();
                            self.drain_stderr(&mut lines).await;
                            let error_context = if saw_error {
                                Self::extract_error_context(&lines)
                            } else {
                                None
                            };
                            return Ok((lines, PromptKind::Debugger { level, error_context }));
                        }
                        // SBCL Lisp debugger prompt (e.g. "0]").
                        if debugger::detect_sbcl_debugger_prompt(trimmed) {
                            self.emit_output(trimmed, "stdout");
                            lines.push(trimmed.to_string());
                            partial.clear();
                            self.drain_stderr(&mut lines).await;
                            let context = lines
                                .iter()
                                .rev()
                                .take(5)
                                .collect::<Vec<_>>()
                                .into_iter()
                                .rev()
                                .map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");
                            return Err(AppError::CommunicationError(format!(
                                "Lisp-level error (SBCL debugger entered):\n{}",
                                context
                            )));
                        }
                    }
                }
                result = self.stderr_reader.read_line(&mut stderr_line) => {
                    let bytes_read = result.map_err(|e| AppError::CommunicationError(e.to_string()))?;
                    if bytes_read == 0 {
                        continue;
                    }
                    let trimmed = stderr_line.trim_end().to_string();
                    if !trimmed.is_empty() {
                        self.emit_output(&trimmed, "stderr");

                        // Check stderr for error markers too — syntax
                        // errors go to *error-output* (stderr) on SBCL.
                        if !saw_error
                            && debugger::ERROR_MARKERS
                                .iter()
                                .any(|m| trimmed.contains(m))
                        {
                            saw_error = true;
                            error_deadline
                                .as_mut()
                                .reset(tokio::time::Instant::now() + error_grace);
                        }

                        lines.push(trimmed);
                    }
                }
                // Error grace period expired — no debugger prompt arrived
                // after a Maxima error marker.  This is a parse error or
                // similar that left Maxima silently waiting for input.
                _ = &mut error_deadline, if saw_error => {
                    self.drain_stderr(&mut lines).await;
                    let context = lines
                        .iter()
                        .rev()
                        .take(5)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    return Err(AppError::CommunicationError(format!(
                        "Maxima error (no debugger recovery):\n{}",
                        context
                    )));
                }
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

pub fn find_maxima_binary() -> String {
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
