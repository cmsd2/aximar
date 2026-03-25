use std::sync::Arc;
use tokio::sync::Mutex;

use aximar_core::catalog::docs::Docs;
use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use aximar_core::registry::NotebookRegistry;

use crate::mcp::McpController;
use crate::tauri_output::AppLog;

pub struct AppState {
    pub registry: Arc<Mutex<NotebookRegistry>>,
    pub catalog: Arc<Catalog>,
    pub docs: Arc<Docs>,
    pub packages: Arc<PackageCatalog>,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
    /// MCP HTTP server lifecycle controller
    pub mcp_controller: Arc<McpController>,
    /// Buffered app-level log entries for frontend replay
    pub app_log: Arc<AppLog>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            registry: Arc::new(Mutex::new(NotebookRegistry::new())),
            catalog: Arc::new(Catalog::load()),
            docs: Arc::new(Docs::load()),
            packages: Arc::new(PackageCatalog::load()),
            app_handle: Arc::new(Mutex::new(None)),
            mcp_controller: Arc::new(McpController::new()),
            app_log: Arc::new(AppLog::new()),
        }
    }
}
