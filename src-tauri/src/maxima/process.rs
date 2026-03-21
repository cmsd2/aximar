use std::env;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::error::AppError;

const READY_SENTINEL: &str = "__AXIMAR_READY__";

pub struct MaximaProcess {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_reader: BufReader<tokio::process::ChildStdout>,
    stderr_reader: BufReader<tokio::process::ChildStderr>,
}

impl MaximaProcess {
    pub async fn spawn() -> Result<Self, AppError> {
        let maxima_path = find_maxima_binary();

        let mut child = Command::new(&maxima_path)
            .arg("--very-quiet")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| AppError::ProcessStartFailed(format!("{}: {}", maxima_path, e)))?;

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
        };

        proc.initialize().await?;
        Ok(proc)
    }

    async fn initialize(&mut self) -> Result<(), AppError> {
        let init_commands = format!(
            "display2d:false$\nset_plot_option([run_viewer, false])$\nset_plot_option([gnuplot_term, svg])$\nprint(\"{}\")$\n",
            READY_SENTINEL
        );

        self.write_stdin(&init_commands).await?;
        self.read_until_sentinel(READY_SENTINEL).await?;

        // Set % to a harmless value so tex(%) after an error in the first cell
        // doesn't render the sentinel string
        self.write_stdin("0$\n").await?;

        Ok(())
    }

    pub async fn write_stdin(&mut self, input: &str) -> Result<(), AppError> {
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
                        lines.push(trimmed);
                    }
                }
                _ => break,
            }
        }
    }

    pub async fn kill(&mut self) -> Result<(), AppError> {
        self.child.kill().await.map_err(AppError::Io)
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
