use serde::Serialize;

/// An output event from the Maxima process (stdin echo, stdout, or stderr).
#[derive(Debug, Clone, Serialize)]
pub struct OutputEvent {
    pub line: String,
    pub stream: String,
    pub timestamp: u64,
}

/// Trait for receiving Maxima output events.
/// Implementations can emit to a Tauri window, capture to a buffer, or discard.
pub trait OutputSink: Send + Sync + 'static {
    fn emit(&self, event: OutputEvent);
}

/// A no-op output sink that discards all events.
pub struct NullOutputSink;

impl OutputSink for NullOutputSink {
    fn emit(&self, _: OutputEvent) {}
}

/// An output sink that broadcasts events to multiple inner sinks.
pub struct MultiOutputSink {
    sinks: Vec<std::sync::Arc<dyn OutputSink>>,
}

impl MultiOutputSink {
    pub fn new(sinks: Vec<std::sync::Arc<dyn OutputSink>>) -> Self {
        MultiOutputSink { sinks }
    }
}

impl OutputSink for MultiOutputSink {
    fn emit(&self, event: OutputEvent) {
        for sink in &self.sinks {
            sink.emit(event.clone());
        }
    }
}
