use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::registry::NotebookRegistry;

use crate::mcp::McpController;
use crate::tauri_output::AppLog;

/// The app handle, set once during setup.  Emitters read it without
/// locking, so no event is skipped because another emitter holds it.
pub type SharedAppHandle = Arc<OnceLock<tauri::AppHandle>>;

pub struct AppState {
    pub registry: Arc<Mutex<NotebookRegistry>>,
    pub catalog: Arc<Catalog>,
    pub packages: Arc<PackageCatalog>,
    pub app_handle: SharedAppHandle,
    /// MCP HTTP server lifecycle controller
    pub mcp_controller: Arc<McpController>,
    /// Buffered app-level log entries for frontend replay
    pub app_log: Arc<AppLog>,
    /// File paths passed as CLI arguments on initial launch (taken once by frontend)
    pub initial_file_args: Arc<Mutex<Option<Vec<String>>>>,
    /// Files the system asked Aximar to open (a drop on the Dock icon,
    /// Open With, a double-click), waiting for the main window to take them
    pub pending_opens: Arc<std::sync::Mutex<Vec<String>>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            registry: Arc::new(Mutex::new(NotebookRegistry::new())),
            catalog: Arc::new(Catalog::load()),
            packages: Arc::new(PackageCatalog::load()),
            app_handle: Arc::new(OnceLock::new()),
            mcp_controller: Arc::new(McpController::new()),
            app_log: Arc::new(AppLog::new()),
            initial_file_args: Arc::new(Mutex::new(None)),
            pending_opens: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
}
