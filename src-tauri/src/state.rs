use std::sync::Arc;
use tokio::sync::Mutex;

use aximar_core::catalog::docs::Docs;
use aximar_core::catalog::search::Catalog;
use aximar_core::session::SessionManager;

use aximar_mcp::capture::CaptureOutputSink;
use aximar_mcp::log::ServerLog;
use aximar_mcp::notebook::McpNotebook;

use crate::mcp::McpController;
use crate::tauri_output::AppLog;

pub struct AppState {
    pub session: Arc<SessionManager>,
    pub catalog: Arc<Catalog>,
    pub docs: Arc<Docs>,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    /// Shared notebook state (mirrored from the frontend, used by MCP)
    pub notebook: Arc<Mutex<McpNotebook>>,
    /// Per-cell output capture for MCP
    pub capture_sink: Arc<CaptureOutputSink>,
    /// Server-wide Maxima output log
    pub server_log: Arc<ServerLog>,
    /// MCP HTTP server lifecycle controller
    pub mcp_controller: Arc<McpController>,
    /// Buffered app-level log entries for frontend replay
    pub app_log: Arc<AppLog>,
}

impl AppState {
    pub fn new() -> Self {
        let server_log = Arc::new(ServerLog::new());
        let capture_sink = Arc::new(CaptureOutputSink::new(server_log.clone()));
        AppState {
            session: Arc::new(SessionManager::new()),
            catalog: Arc::new(Catalog::load()),
            docs: Arc::new(Docs::load()),
            app_handle: Arc::new(Mutex::new(None)),
            notebook: Arc::new(Mutex::new(McpNotebook::new())),
            capture_sink,
            server_log,
            mcp_controller: Arc::new(McpController::new()),
            app_log: Arc::new(AppLog::new()),
        }
    }
}
