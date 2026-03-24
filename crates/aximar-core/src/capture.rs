use std::sync::Mutex;

use crate::log::ServerLog;
use crate::maxima::output::{OutputEvent, OutputSink};

/// OutputSink that captures Maxima I/O per-cell and also logs to a server-wide buffer.
pub struct CaptureOutputSink {
    /// Raw output for the currently-evaluating cell.
    current_cell_output: Mutex<Vec<OutputEvent>>,
    /// Server-wide log ring buffer.
    server_log: std::sync::Arc<ServerLog>,
}

impl CaptureOutputSink {
    pub fn new(server_log: std::sync::Arc<ServerLog>) -> Self {
        CaptureOutputSink {
            current_cell_output: Mutex::new(Vec::new()),
            server_log,
        }
    }

    /// Take the captured output for the current cell (and clear the buffer).
    pub fn take_cell_output(&self) -> Vec<OutputEvent> {
        let mut guard = self.current_cell_output.lock().unwrap();
        std::mem::take(&mut *guard)
    }
}

impl OutputSink for CaptureOutputSink {
    fn emit(&self, event: OutputEvent) {
        // Log to server-wide log
        self.server_log.push(event.clone());

        // Capture for current cell
        if let Ok(mut guard) = self.current_cell_output.lock() {
            guard.push(event);
        }
    }
}
