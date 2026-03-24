use std::sync::Mutex;

use crate::maxima::output::OutputEvent;

const MAX_LOG_ENTRIES: usize = 5000;

/// Ring buffer for server-wide Maxima output logging.
pub struct ServerLog {
    entries: Mutex<Vec<OutputEvent>>,
}

impl ServerLog {
    pub fn new() -> Self {
        ServerLog {
            entries: Mutex::new(Vec::new()),
        }
    }

    pub fn push(&self, event: OutputEvent) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(event);
            if entries.len() > MAX_LOG_ENTRIES {
                let drain = entries.len() - MAX_LOG_ENTRIES;
                entries.drain(..drain);
            }
        }
    }

    pub fn get(&self, limit: Option<usize>, stream_filter: Option<&str>) -> Vec<OutputEvent> {
        let entries = self.entries.lock().unwrap();
        let iter = entries.iter().filter(|e| {
            stream_filter.map_or(true, |f| e.stream == f)
        });

        match limit {
            Some(n) => iter.rev().take(n).cloned().collect::<Vec<_>>().into_iter().rev().collect(),
            None => iter.cloned().collect(),
        }
    }
}
