use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Maxima process not running")]
    ProcessNotRunning,

    #[error("Session is busy with another evaluation")]
    SessionBusy,

    #[error("Failed to start Maxima: {0}")]
    ProcessStartFailed(String),

    #[error("Failed to communicate with Maxima: {0}")]
    CommunicationError(String),

    #[error("Evaluation timed out after {0} seconds")]
    Timeout(u64),

    #[error("Maxima needs more information: \"{0}\". Add assume() declarations before this computation, e.g. assume(x > 0);")]
    AssumptionRequired(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cannot run a markdown cell")]
    CellIsMarkdown,

    #[error("Cell input is empty")]
    EmptyInput,

    #[error("Cell '{0}' not found")]
    CellNotFound(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
