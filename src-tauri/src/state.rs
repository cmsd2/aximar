use std::sync::Arc;
use tokio::sync::Mutex;

use crate::catalog::docs::Docs;
use crate::catalog::search::Catalog;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::SessionStatus;

pub struct AppState {
    pub process: Arc<Mutex<Option<MaximaProcess>>>,
    pub status: Arc<Mutex<SessionStatus>>,
    pub catalog: Catalog,
    pub docs: Docs,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            process: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(SessionStatus::Stopped)),
            catalog: Catalog::load(),
            docs: Docs::load(),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }
}
