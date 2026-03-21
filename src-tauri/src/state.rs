use std::sync::Arc;
use tokio::sync::Mutex;

use crate::catalog::search::Catalog;
use crate::maxima::process::MaximaProcess;
use crate::maxima::types::SessionStatus;

pub struct AppState {
    pub process: Arc<Mutex<Option<MaximaProcess>>>,
    pub status: Arc<Mutex<SessionStatus>>,
    pub catalog: Catalog,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            process: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(SessionStatus::Stopped)),
            catalog: Catalog::load(),
        }
    }
}
