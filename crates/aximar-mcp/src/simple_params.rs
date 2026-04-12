use schemars::JsonSchema;
use serde::Deserialize;

// ── Simple mode parameter types (session-oriented naming) ─────────────

/// Used by tools that target an optional session.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SessionIdParam {
    /// Session to target (defaults to the default session if omitted)
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimpleEvaluateExpressionParams {
    /// Maxima expression to evaluate
    pub expression: String,
    /// Session to target (defaults to the default session if omitted)
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimpleKillVariableParams {
    /// Variable name to kill
    pub name: String,
    /// Session to target (defaults to the default session if omitted)
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CreateSessionParams {
    /// Optional filesystem path for the working directory (e.g. the notebook's parent directory).
    /// When set, the Maxima process will use this as its current directory so that
    /// relative file paths in load() and batchload() resolve correctly.
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CloseSessionParams {
    /// ID of the session to close
    pub session_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SimpleGetServerLogParams {
    /// Filter by stream: "stdout", "stderr", or "stdin"
    pub stream: Option<String>,
    /// Maximum number of entries to return (default: all)
    pub limit: Option<usize>,
    /// Session to target (defaults to the default session if omitted)
    pub session_id: Option<String>,
}
