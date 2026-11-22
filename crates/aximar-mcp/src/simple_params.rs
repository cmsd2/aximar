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
    /// Run the expression as this cell of a notebook the client keeps
    /// itself, such as an editor's notebook view: the result's
    /// execution_count is its gutter number, %oN means that notebook's
    /// Out[N], and a bare % at the start means the output of the nearest
    /// cell above that has one in this session. Use an ID that stays the
    /// same while the notebook is open
    pub cell_id: Option<String>,
    /// With cell_id: the IDs of the code cells above it, in order
    #[serde(default)]
    pub cells_above: Vec<String>,
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
