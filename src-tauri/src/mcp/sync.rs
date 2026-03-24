use serde::{Deserialize, Serialize};
use tauri::State;

use aximar_core::error::AppError;
use aximar_mcp::notebook::{CellOutput, CellStatus, CellType, McpNotebook};

use crate::state::AppState;

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
}

/// Payload emitted to the frontend when MCP modifies the notebook.
#[derive(Debug, Clone, Serialize)]
pub struct NotebookSyncPayload {
    pub cells: Vec<SyncCell>,
}

/// Build a sync payload from the current notebook state.
pub fn notebook_state_payload(nb: &McpNotebook) -> NotebookSyncPayload {
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
            }),
        })
        .collect();
    NotebookSyncPayload { cells }
}

/// Tauri command: frontend pushes its current cell state to the backend's
/// McpNotebook so that MCP sees up-to-date content.
#[tauri::command]
pub async fn sync_notebook_state(
    state: State<'_, AppState>,
    cells: Vec<SyncCell>,
) -> Result<(), AppError> {
    let mut nb = state.notebook.lock().await;
    nb.clear();

    for (i, cell) in cells.iter().enumerate() {
        let cell_type = match cell.cell_type.as_str() {
            "markdown" => CellType::Markdown,
            _ => CellType::Code,
        };

        if i == 0 {
            // Update the initial cell created by clear()
            let first_id = nb.cells().first().map(|c| c.id.clone());
            if let Some(first_id) = first_id {
                nb.update_cell(&first_id, Some(cell.input.clone()), Some(cell_type));
            }
        } else {
            nb.add_cell(cell_type, cell.input.clone(), None);
        }
    }

    Ok(())
}
