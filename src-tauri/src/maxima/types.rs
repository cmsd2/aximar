use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub cell_id: String,
    pub text_output: String,
    pub latex: Option<String>,
    pub plot_svg: Option<String>,
    pub error: Option<String>,
    pub error_info: Option<ErrorInfo>,
    pub is_error: bool,
    pub duration_ms: u64,
    /// Maxima output label (e.g. "%o6") for stable back-references
    pub output_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub title: String,
    pub explanation: String,
    pub suggestion: Option<String>,
    pub example: Option<String>,
    pub did_you_mean: Vec<String>,
    pub correct_signatures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Starting,
    Ready,
    Busy,
    Stopped,
    Error(String),
}

impl SessionStatus {
    pub(crate) fn as_code(&self) -> u8 {
        match self {
            SessionStatus::Stopped => 0,
            SessionStatus::Starting => 1,
            SessionStatus::Ready => 2,
            SessionStatus::Busy => 3,
            SessionStatus::Error(_) => 4,
        }
    }

    pub(crate) fn from_code(code: u8, error_msg: impl FnOnce() -> String) -> Self {
        match code {
            1 => SessionStatus::Starting,
            2 => SessionStatus::Ready,
            3 => SessionStatus::Busy,
            4 => SessionStatus::Error(error_msg()),
            _ => SessionStatus::Stopped,
        }
    }
}
