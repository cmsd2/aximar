use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub cell_id: String,
    pub text_output: String,
    pub latex: Option<String>,
    pub plot_svg: Option<String>,
    pub error: Option<String>,
    pub is_error: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Starting,
    Ready,
    Busy,
    Stopped,
    Error(String),
}
