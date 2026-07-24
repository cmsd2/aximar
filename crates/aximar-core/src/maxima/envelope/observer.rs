//! Live observation of the kernel-events fd-3 stream.
//!
//! The protocol layer drains envelopes from the same channel to drive
//! its eval reads — this trait lets a *parallel* observer (the GUI's
//! "Events" log tab) see every line as it lands, including lines that
//! failed to parse as an envelope.  Tee'd inside the fd-3 reader task
//! so the observer sees frames in arrival order without depending on
//! when the protocol layer drains.

use serde::Serialize;

/// One frame read from fd-3.  Carries the raw JSON line so the
/// observer can pretty-print it without needing every envelope type
/// to derive `Serialize`; `kind` is the parsed envelope label (or
/// `None` for parse failures) so the GUI can colour/filter without
/// re-parsing.
#[derive(Debug, Clone, Serialize)]
pub struct EnvelopeFrame {
    /// Milliseconds since UNIX epoch when the line was read.
    pub timestamp_ms: u64,
    /// The raw JSON line as read from fd-3 (trailing newline trimmed).
    pub raw_line: String,
    /// Envelope kind label (`"eval_end"`, `"vars"`, ...) when the line
    /// parsed successfully; `None` when parsing failed.
    pub kind: Option<String>,
    /// Parse error message when `kind` is `None`.
    pub parse_error: Option<String>,
}

/// Observer interface for the fd-3 envelope stream.  Implementations
/// receive every frame the reader task pulls off the pipe.
pub trait EnvelopeObserver: Send + Sync + 'static {
    fn observe(&self, frame: EnvelopeFrame);
}

/// No-op observer for callers that don't need GUI visibility (MCP,
/// integration tests).  Passing `None` for the observer at spawn time
/// is equivalent to using this.
pub struct NullEnvelopeObserver;

impl EnvelopeObserver for NullEnvelopeObserver {
    fn observe(&self, _: EnvelopeFrame) {}
}

/// Current wall-clock time in milliseconds since epoch, suitable for
/// `EnvelopeFrame.timestamp_ms`.  Saturates to 0 if the system clock
/// is before the epoch (effectively impossible).
pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
