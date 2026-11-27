use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tokio_util::sync::CancellationToken;

/// What the embedded MCP server is doing, for the Settings window
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum McpServerState {
    Off,
    Starting {
        address: String,
    },
    Listening {
        address: String,
    },
    /// It couldn't listen, such as when another program holds the port
    Failed {
        address: String,
        reason: String,
    },
}

type OnChange = Box<dyn Fn(&McpServerState) + Send + Sync>;

/// Manages the lifecycle of the embedded MCP HTTP server.
///
/// Holds a `CancellationToken` for the currently running server (if any),
/// allowing start/stop/restart without restarting the whole app, and the
/// server's state.  Each start gets a new generation, so a stopped
/// server finishing late can't overwrite the state of the one after it.
pub struct McpController {
    token: tokio::sync::Mutex<Option<CancellationToken>>,
    state: Mutex<(u64, McpServerState)>,
    on_change: OnceLock<OnChange>,
}

impl McpController {
    pub fn new() -> Self {
        McpController {
            token: tokio::sync::Mutex::new(None),
            state: Mutex::new((0, McpServerState::Off)),
            on_change: OnceLock::new(),
        }
    }

    /// Call `f` with each new state, such as to tell the window
    pub fn on_change(&self, f: impl Fn(&McpServerState) + Send + Sync + 'static) {
        let _ = self.on_change.set(Box::new(f));
    }

    fn publish(&self, state: &McpServerState) {
        if let Some(f) = self.on_change.get() {
            f(state);
        }
    }

    /// Stop the currently running MCP server (if any).
    pub async fn stop(&self) {
        if let Some(ct) = self.token.lock().await.take() {
            ct.cancel();
        }
        let state = {
            let mut guard = self.state.lock().unwrap();
            guard.0 += 1;
            guard.1 = McpServerState::Off;
            guard.1.clone()
        };
        self.publish(&state);
    }

    /// Record that a new server instance is starting on `address` with the
    /// given cancellation token.  Returns its generation, for `set_state`.
    pub async fn set_running(&self, ct: CancellationToken, address: &str) -> u64 {
        *self.token.lock().await = Some(ct);
        let (generation, state) = {
            let mut guard = self.state.lock().unwrap();
            guard.0 += 1;
            guard.1 = McpServerState::Starting {
                address: address.to_string(),
            };
            (guard.0, guard.1.clone())
        };
        self.publish(&state);
        generation
    }

    /// The server of `generation` moved to `state`.  Ignored when a later
    /// start or stop has happened since.
    pub fn set_state(&self, generation: u64, state: McpServerState) {
        {
            let mut guard = self.state.lock().unwrap();
            if guard.0 != generation {
                return;
            }
            guard.1 = state.clone();
        }
        self.publish(&state);
    }

    /// What the server is doing now
    pub fn state(&self) -> McpServerState {
        self.state.lock().unwrap().1.clone()
    }

    /// Check whether the MCP server is currently running.
    pub async fn is_running(&self) -> bool {
        self.token.lock().await.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn a_bind_failure_is_recorded() {
        let controller = McpController::new();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        controller.on_change(move |s| log.lock().unwrap().push(s.clone()));

        // Hold a port, so the server can't have it
        let held = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = held.local_addr().unwrap().to_string();
        let generation = controller
            .set_running(CancellationToken::new(), &address)
            .await;
        let reason = tokio::net::TcpListener::bind(&address)
            .await
            .expect_err("the port is held")
            .to_string();
        controller.set_state(
            generation,
            McpServerState::Failed {
                address: address.clone(),
                reason: reason.clone(),
            },
        );

        assert_eq!(
            controller.state(),
            McpServerState::Failed { address, reason }
        );
        assert_eq!(seen.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_late_stop_leaves_the_next_server_alone() {
        let controller = McpController::new();
        let first = controller
            .set_running(CancellationToken::new(), "127.0.0.1:1")
            .await;
        let second = controller
            .set_running(CancellationToken::new(), "127.0.0.1:2")
            .await;
        controller.set_state(
            second,
            McpServerState::Listening {
                address: "127.0.0.1:2".into(),
            },
        );
        // The first server finishes after the second started
        controller.set_state(first, McpServerState::Off);
        assert_eq!(
            controller.state(),
            McpServerState::Listening {
                address: "127.0.0.1:2".into()
            }
        );
    }
}
