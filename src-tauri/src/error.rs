use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Maxima process not running")]
    ProcessNotRunning,

    #[error("Failed to start Maxima: {0}")]
    ProcessStartFailed(String),

    #[error("Failed to communicate with Maxima: {0}")]
    CommunicationError(String),

    #[error("Evaluation timed out after {0} seconds")]
    Timeout(u64),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
