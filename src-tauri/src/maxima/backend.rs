use std::path::PathBuf;

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
                // WSL writes to /tmp/maxoutXXX.svg
                // Host reads from \\wsl.localhost\<distro>\tmp\maxoutXXX.svg
                if distro.is_empty() {
                    None
                } else {
                    let wsl_path = format!(
                        "\\\\wsl.localhost\\{}{}",
                        distro,
                        container_path.replace('/', "\\")
                    );
                    Some(wsl_path)
                }
            }
        }
    }

    /// Host directory that gets volume-mounted into the Docker container.
    pub fn host_temp_dir(&self) -> Option<PathBuf> {
        match self {
            Backend::Docker { .. } => {
                Some(std::env::temp_dir().join("aximar-docker"))
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
