use std::sync::Arc;
use tokio::sync::Mutex;

use crate::catalog::docs::Docs;
use crate::catalog::search::Catalog;
use crate::session::SessionManager;

pub struct AppState {
    pub session: SessionManager,
    pub catalog: Catalog,
    pub docs: Docs,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            session: SessionManager::new(),
            catalog: Catalog::load(),
            docs: Docs::load(),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }
}
