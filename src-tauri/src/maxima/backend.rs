use std::path::PathBuf;
use std::process::Command;

use crate::maxima::noconsole::hide_console_window_sync;

const CONTAINER_TEMP_DIR: &str = "/tmp/aximar";

/// Docker's default seccomp profile with additional `personality` syscall values
/// required by GCL (GNU Common Lisp), which is the Lisp runtime used by
/// Ubuntu's Maxima package. GCL needs ADDR_NO_RANDOMIZE (0x40000) and
/// READ_IMPLIES_EXEC (0x400000), which Docker's default profile blocks.
const SECCOMP_PROFILE: &str = include_str!("../../../docker/seccomp.json");

#[derive(Debug, Clone)]
pub enum Backend {
    Local,
    Docker {
        engine: String,
        image: String,
    },
    Wsl {
        distro: String,
    },
}

impl Backend {
    pub fn from_config(
        backend: &str,
        docker_image: &str,
        wsl_distro: &str,
        container_engine: &str,
    ) -> Self {
        match backend {
            "docker" => Backend::Docker {
                engine: container_engine.to_string(),
                image: docker_image.to_string(),
            },
            "wsl" => Backend::Wsl {
                distro: wsl_distro.to_string(),
            },
            _ => Backend::Local,
        }
    }

    /// Translate a container/WSL SVG path to the host-accessible path.
    pub fn translate_svg_path(&self, container_path: &str) -> Option<String> {
        match self {
            Backend::Local => None,
            Backend::Docker { .. } => {
                // Container writes to /tmp/aximar/maxoutXXX.svg
                // Host reads from <host_temp>/aximar-docker/maxoutXXX.svg
                if let Some(filename) = container_path.strip_prefix(CONTAINER_TEMP_DIR) {
                    let filename = filename.trim_start_matches('/');
                    if let Some(host_dir) = self.host_temp_dir() {
                        return Some(host_dir.join(filename).to_string_lossy().to_string());
                    }
                }
                None
            }
            Backend::Wsl { distro } => {
                // WSL writes to /tmp/aximar/maxout.svg
                // We copy it to a local temp dir with a unique name so plots
                // don't overwrite each other and we avoid UNC path issues.
                if let Some(filename) = container_path.strip_prefix(CONTAINER_TEMP_DIR) {
                    let filename = filename.trim_start_matches('/');
                    if let Some(host_dir) = self.host_temp_dir() {
                        let unique_name = format!(
                            "{}-{}.svg",
                            filename.trim_end_matches(".svg"),
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_nanos())
                                .unwrap_or(0)
                        );
                        let host_path = host_dir.join(&unique_name);

                        // Build the UNC source path
                        let effective_distro = if distro.is_empty() {
                            resolve_default_wsl_distro().unwrap_or_default()
                        } else {
                            distro.clone()
                        };
                        if effective_distro.is_empty() {
                            return None;
                        }
                        let unc_path = format!(
                            "\\\\wsl.localhost\\{}{}",
                            effective_distro,
                            container_path.replace('/', "\\")
                        );

                        // Copy from WSL to local temp
                        let _ = std::fs::create_dir_all(&host_dir);
                        if std::fs::copy(&unc_path, &host_path).is_ok() {
                            return Some(host_path.to_string_lossy().to_string());
                        }
                    }
                }
                None
            }
        }
    }

    /// Host directory where plot SVGs are accessible.
    /// For Docker this is the volume-mount source; for WSL it holds copies
    /// fetched from the WSL filesystem.
    pub fn host_temp_dir(&self) -> Option<PathBuf> {
        match self {
            Backend::Docker { .. } => {
                Some(std::env::temp_dir().join("aximar-docker"))
            }
            Backend::Wsl { .. } => {
                Some(std::env::temp_dir().join("aximar-wsl"))
            }
            _ => None,
        }
    }

    /// The fixed mount target inside the Docker container.
    pub fn container_temp_dir() -> &'static str {
        CONTAINER_TEMP_DIR
    }

    /// Write the embedded seccomp profile to a temp file and return its path.
    /// The file persists for the lifetime of the process.
    pub fn write_seccomp_profile() -> Result<PathBuf, std::io::Error> {
        let path = std::env::temp_dir().join("aximar-seccomp.json");
        std::fs::write(&path, SECCOMP_PROFILE)?;
        Ok(path)
    }
}

/// Query WSL for the default distribution name.
/// `wsl --status` outputs UTF-16LE on Windows; we decode and parse it.
fn resolve_default_wsl_distro() -> Option<String> {
    let mut cmd = Command::new("wsl");
    cmd.arg("--status")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    hide_console_window_sync(&mut cmd);
    let output = cmd.output().ok()?;

    // wsl --status outputs UTF-16LE on Windows
    let text = decode_wsl_output(&output.stdout);

    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Default Distribution:") {
            let name = rest.trim();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Decode WSL output which may be UTF-16LE (with or without BOM) or UTF-8.
pub fn decode_wsl_output(bytes: &[u8]) -> String {
    // Check for UTF-16LE BOM or the NUL-interleaved pattern typical of UTF-16LE ASCII
    if bytes.len() >= 2 && (bytes[0] == 0xFF && bytes[1] == 0xFE) {
        // Has BOM — skip it
        decode_utf16le(&bytes[2..])
    } else if bytes.len() >= 2 && bytes[1] == 0 {
        // No BOM but looks like UTF-16LE (second byte is NUL for ASCII range)
        decode_utf16le(bytes)
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

fn decode_utf16le(bytes: &[u8]) -> String {
    let iter = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));
    char::decode_utf16(iter)
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}
