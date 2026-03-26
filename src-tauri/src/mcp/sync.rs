use serde::{Deserialize, Serialize};

use aximar_core::notebook::{CellOutput, CellStatus, CellType, Notebook};

/// Serializable cell state for syncing between frontend and backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCell {
    pub id: String,
    pub cell_type: String,
    pub input: String,
    /// Cell output (only present in backend → frontend direction).
    #[serde(default, skip_deserializing)]
    pub output: Option<CellOutput>,
    /// Cell status (only present in backend → frontend direction).
    #[serde(default, skip_deserializing)]
    pub status: Option<String>,
    /// Dangerous functions detected (only when status is pending_approval).
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub dangerous_functions: Option<Vec<String>>,
    /// Whether the user has trusted this cell's content.
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub trusted: Option<bool>,
}

/// Payload containing cell state for events.
#[derive(Debug, Clone, Serialize)]
pub struct NotebookSyncPayload {
    pub cells: Vec<SyncCell>,
}

/// Build a sync payload from the current notebook state.
pub fn notebook_state_payload(nb: &Notebook) -> NotebookSyncPayload {
    let cells = nb
        .cells()
        .iter()
        .map(|c| SyncCell {
            id: c.id.clone(),
            cell_type: match c.cell_type {
                CellType::Code => "code".to_string(),
                CellType::Markdown => "markdown".to_string(),
            },
            input: c.input.clone(),
            output: c.output.clone(),
            status: Some(match c.status {
                CellStatus::Idle => "idle".to_string(),
                CellStatus::Running => "running".to_string(),
                CellStatus::Success => "success".to_string(),
                CellStatus::Error => "error".to_string(),
                CellStatus::PendingApproval => "pending_approval".to_string(),
            }),
            dangerous_functions: c.dangerous_functions.clone(),
            trusted: Some(c.trusted),
        })
        .collect();
    NotebookSyncPayload { cells }
}
