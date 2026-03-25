use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::notebook::{CellStatus, CellType};

// ── Tool parameter types ──────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SearchFunctionsParams {
    /// Search query (matches function name and description)
    pub query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetFunctionDocsParams {
    /// Function name (case-insensitive)
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CompleteFunctionParams {
    /// Prefix to complete (e.g. "integ" → "integrate")
    pub prefix: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SearchPackagesParams {
    /// Search query (matches package names and descriptions)
    pub query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetPackageParams {
    /// Package name (e.g. "distrib", "simplification/absimp")
    pub name: String,
}

/// Used by tools that only need an optional notebook_id.
#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct NotebookIdParam {
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CellIdParams {
    /// Cell ID
    pub cell_id: String,
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct AddCellParams {
    /// Cell type: "code" or "markdown" (default: "code")
    pub cell_type: Option<String>,
    /// Initial cell content
    pub input: Option<String>,
    /// Insert after this cell ID (appends to end if omitted)
    pub after_cell_id: Option<String>,
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct UpdateCellParams {
    /// Cell ID to update
    pub cell_id: String,
    /// New cell content
    pub input: Option<String>,
    /// New cell type: "code" or "markdown"
    pub cell_type: Option<String>,
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct MoveCellParams {
    /// Cell ID to move
    pub cell_id: String,
    /// Direction: "up" or "down"
    pub direction: String,
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct EvaluateExpressionParams {
    /// Maxima expression to evaluate
    pub expression: String,
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct KillVariableParams {
    /// Variable name to kill
    pub name: String,
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct GetServerLogParams {
    /// Filter by stream: "stdout", "stderr", or "stdin"
    pub stream: Option<String>,
    /// Maximum number of entries to return (default: all)
    pub limit: Option<usize>,
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct NotebookPathParams {
    /// File path for the notebook (.ipynb)
    pub path: String,
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct LoadTemplateParams {
    /// Template ID (see list_templates)
    pub template_id: String,
    /// Notebook to target (defaults to active notebook if omitted)
    pub notebook_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct CloseNotebookParams {
    /// ID of the notebook to close
    pub notebook_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct SwitchNotebookParams {
    /// ID of the notebook to switch to
    pub notebook_id: String,
}

// ── Tool result helpers ───────────────────────────────────────────────

/// Return a successful JSON-serialized result.
/// Using Result<String, String> because rmcp's IntoCallToolResult maps
/// Ok(String) → CallToolResult::success and Err(String) → CallToolResult::error.
pub(crate) fn success_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| format!("Serialization error: {e}"))
}

pub(crate) fn error_result(msg: impl Into<String>) -> Result<String, String> {
    Err(msg.into())
}

// ── Cell serialization for tool responses ─────────────────────────────

#[derive(Serialize)]
pub(crate) struct CellSummary {
    pub id: String,
    pub cell_type: CellType,
    pub input: String,
    pub status: CellStatus,
    pub has_output: bool,
    pub output_preview: Option<String>,
}
