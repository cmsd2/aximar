use serde::{Deserialize, Serialize};

use aximar_core::maxima::protocol::ends_silent;
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
    /// Whether the input's last statement ends in `$`, so the cell shows no
    /// result by design (backend → frontend; worked out from the input, not
    /// stored)
    #[serde(default, skip_deserializing)]
    pub silent: bool,
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
                CellType::Raw => "raw".to_string(),
            },
            input: c.input.clone(),
            silent: c.cell_type == CellType::Code && ends_silent(&c.input),
            output: c.output.clone(),
            status: Some(match c.status {
                CellStatus::Idle => "idle".to_string(),
                CellStatus::Queued => "queued".to_string(),
                CellStatus::Running => "running".to_string(),
                CellStatus::Success => "success".to_string(),
                CellStatus::Error => "error".to_string(),
            }),
        })
        .collect();
    NotebookSyncPayload { cells }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aximar_core::commands::NotebookCommand;

    fn payload_for(inputs: &[&str]) -> NotebookSyncPayload {
        let mut nb = Notebook::empty();
        for input in inputs {
            nb.apply(NotebookCommand::AddCell {
                cell_type: CellType::Code,
                input: input.to_string(),
                after_cell_id: None,
                before_cell_id: None,
            })
            .unwrap();
        }
        notebook_state_payload(&nb)
    }

    #[test]
    fn cells_ending_in_dollar_are_silent() {
        let cells = payload_for(&["f(x) := x^2 - 2$", "x^2;", "g(x) := x$ /* helper */"]).cells;
        assert_eq!(
            cells.iter().map(|c| c.silent).collect::<Vec<_>>(),
            vec![true, false, true]
        );
    }
}
