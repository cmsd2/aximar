use std::sync::{Arc, Mutex as StdMutex};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::sync::Mutex;

use aximar_core::maxima::output::{OutputEvent, OutputSink};

/// OutputSink implementation that emits events to the Tauri frontend.
pub struct TauriOutputSink {
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl TauriOutputSink {
    pub fn new(app_handle: Arc<Mutex<Option<tauri::AppHandle>>>) -> Self {
        TauriOutputSink { app_handle }
    }
}

impl OutputSink for TauriOutputSink {
    fn emit(&self, event: OutputEvent) {
        // Try to get the app handle without blocking. If the lock is held,
        // we skip the event rather than blocking the Maxima I/O loop.
        if let Ok(guard) = self.app_handle.try_lock() {
            if let Some(ref handle) = *guard {
                let _ = handle.emit("maxima-output", event);
            }
        }
    }
}

/// Structured app log event emitted to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLogEvent {
    pub level: String,
    pub message: String,
    pub source: String,
}

/// Buffer for app-level log entries emitted before the frontend is ready.
///
/// The frontend drains this buffer on mount so that early log entries
/// (e.g. MCP server startup) are not lost.
pub struct AppLog {
    entries: StdMutex<Vec<AppLogEvent>>,
}

impl AppLog {
    pub fn new() -> Self {
        AppLog {
            entries: StdMutex::new(Vec::new()),
        }
    }

    fn push(&self, event: AppLogEvent) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(event);
        }
    }

    /// Take all buffered entries, leaving the buffer empty.
    pub fn drain(&self) -> Vec<AppLogEvent> {
        if let Ok(mut entries) = self.entries.lock() {
            std::mem::take(&mut *entries)
        } else {
            Vec::new()
        }
    }
}

/// Emit an app-level log entry to the frontend via Tauri event,
/// and buffer it in `app_log` so late-mounting frontends can replay it.
pub fn emit_app_log(
    app_handle: &Arc<Mutex<Option<tauri::AppHandle>>>,
    app_log: &AppLog,
    level: &str,
    message: &str,
    source: &str,
) {
    let event = AppLogEvent {
        level: level.to_string(),
        message: message.to_string(),
        source: source.to_string(),
    };
    app_log.push(event.clone());
    if let Ok(guard) = app_handle.try_lock() {
        if let Some(ref handle) = *guard {
            let _ = handle.emit("app-log", event);
        }
    }
}
