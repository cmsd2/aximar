use aximar_core::maxima::backend::{Backend, BackendKind, ContainerEngine};

/// Build a Backend from environment variables.
pub fn backend_from_env() -> Backend {
    let kind = match std::env::var("AXIMAR_BACKEND").as_deref() {
        Ok("docker") => BackendKind::Docker,
        Ok("wsl") => BackendKind::Wsl,
        _ => BackendKind::Local,
    };

    let docker_image = std::env::var("AXIMAR_DOCKER_IMAGE").unwrap_or_default();
    let wsl_distro = std::env::var("AXIMAR_WSL_DISTRO").unwrap_or_default();

    let container_engine = match std::env::var("AXIMAR_CONTAINER_ENGINE").as_deref() {
        Ok("podman") => ContainerEngine::Podman,
        _ => ContainerEngine::Docker,
    };

    Backend::from_config(kind, &docker_image, &wsl_distro, container_engine)
}

pub fn maxima_path_from_env() -> Option<String> {
    std::env::var("AXIMAR_MAXIMA_PATH").ok().filter(|p| !p.is_empty())
}

pub fn eval_timeout_from_env() -> u64 {
    std::env::var("AXIMAR_EVAL_TIMEOUT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30)
}
