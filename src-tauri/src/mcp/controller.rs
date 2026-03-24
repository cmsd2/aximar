use tokio_util::sync::CancellationToken;

/// Manages the lifecycle of the embedded MCP HTTP server.
///
/// Holds a `CancellationToken` for the currently running server (if any),
/// allowing start/stop/restart without restarting the whole app.
pub struct McpController {
    token: tokio::sync::Mutex<Option<CancellationToken>>,
}

impl McpController {
    pub fn new() -> Self {
        McpController {
            token: tokio::sync::Mutex::new(None),
        }
    }

    /// Stop the currently running MCP server (if any).
    pub async fn stop(&self) {
        if let Some(ct) = self.token.lock().await.take() {
            ct.cancel();
        }
    }

    /// Record that a new server instance is running with the given cancellation token.
    pub async fn set_running(&self, ct: CancellationToken) {
        *self.token.lock().await = Some(ct);
    }

    /// Check whether the MCP server is currently running.
    pub async fn is_running(&self) -> bool {
        self.token.lock().await.is_some()
    }
}
