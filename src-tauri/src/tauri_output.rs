use serde::{Deserialize, Serialize};
use std::sync::Mutex as StdMutex;
use tauri::Emitter;

use crate::state::SharedAppHandle;

use aximar_core::maxima::envelope::{EnvelopeFrame, EnvelopeObserver};
use aximar_core::maxima::output::{OutputEvent, OutputSink};

/// Tagged output event that includes the notebook ID for routing.
#[derive(Debug, Clone, Serialize)]
struct TaggedOutputEvent {
    notebook_id: String,
    line: String,
    stream: String,
    timestamp: u64,
}

/// OutputSink implementation that emits events to the Tauri frontend,
/// tagged with a notebook ID so the frontend can route to the correct tab.
pub struct TauriOutputSink {
    app_handle: SharedAppHandle,
    notebook_id: Option<String>,
}

impl TauriOutputSink {
    #[allow(dead_code)]
    pub fn new(app_handle: SharedAppHandle) -> Self {
        TauriOutputSink {
            app_handle,
            notebook_id: None,
        }
    }

    pub fn with_notebook_id(app_handle: SharedAppHandle, notebook_id: String) -> Self {
        TauriOutputSink {
            app_handle,
            notebook_id: Some(notebook_id),
        }
    }
}

impl OutputSink for TauriOutputSink {
    fn emit(&self, event: OutputEvent) {
        if let Some(handle) = self.app_handle.get() {
            if let Some(ref nb_id) = self.notebook_id {
                let tagged = TaggedOutputEvent {
                    notebook_id: nb_id.clone(),
                    line: event.line,
                    stream: event.stream,
                    timestamp: event.timestamp,
                };
                let _ = handle.emit("maxima-output", tagged);
            } else {
                let _ = handle.emit("maxima-output", event);
            }
        }
    }
}

/// Tagged envelope frame that includes the notebook ID for routing.
#[derive(Debug, Clone, Serialize)]
struct TaggedEnvelopeFrame {
    notebook_id: String,
    #[serde(flatten)]
    frame: EnvelopeFrame,
}

/// EnvelopeObserver implementation that emits frames to the Tauri
/// frontend as `maxima-event` events.  Tagged with a notebook ID so
/// the frontend can route to the correct tab's events stream.
pub struct TauriEnvelopeObserver {
    app_handle: SharedAppHandle,
    notebook_id: String,
}

impl TauriEnvelopeObserver {
    pub fn new(app_handle: SharedAppHandle, notebook_id: String) -> Self {
        TauriEnvelopeObserver {
            app_handle,
            notebook_id,
        }
    }
}

impl EnvelopeObserver for TauriEnvelopeObserver {
    fn observe(&self, frame: EnvelopeFrame) {
        if let Some(handle) = self.app_handle.get() {
            let tagged = TaggedEnvelopeFrame {
                notebook_id: self.notebook_id.clone(),
                frame,
            };
            let _ = handle.emit("maxima-event", tagged);
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
    app_handle: &SharedAppHandle,
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
    if let Some(handle) = app_handle.get() {
        let _ = handle.emit("app-log", event);
    }
}
