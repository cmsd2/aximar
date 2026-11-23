use serde::Serialize;

use crate::maxima::output::OutputEvent;
use crate::notebook::{CellOutput, CellStatus, CellType, Notebook};
use crate::notebooks::format::NotebookFile;

/// A command representing a structural mutation to the notebook.
/// All notebook state changes flow through commands for consistency.
#[derive(Debug, Clone)]
pub enum NotebookCommand {
    AddCell {
        cell_type: CellType,
        input: String,
        after_cell_id: Option<String>,
        before_cell_id: Option<String>,
    },
    DeleteCell {
        cell_id: String,
    },
    MoveCell {
        cell_id: String,
        direction: String,
    },
    ToggleCellType {
        cell_id: String,
    },
    UpdateCellInput {
        cell_id: String,
        input: String,
    },
    /// Non-undoable: set cell execution status
    SetCellStatus {
        cell_id: String,
        status: CellStatus,
    },
    /// Non-undoable: set cell output after evaluation
    SetCellOutput {
        cell_id: String,
        output: Box<CellOutput>,
        raw_output: Vec<OutputEvent>,
    },
    /// Non-undoable: set notebook-level trust flag
    TrustNotebook {
        trusted: bool,
    },
    NewNotebook,
    LoadCells {
        cells: Vec<(String, CellType, String, Option<CellOutput>)>,
    },
    /// Replace the notebook with one read from a file
    /// (`notebooks::format::read`), keeping its metadata.
    LoadNotebook {
        file: Box<NotebookFile>,
    },
    Undo,
    Redo,
}

/// The effect produced by applying a command, describing what changed.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandEffect {
    CellAdded {
        cell_id: String,
    },
    CellDeleted {
        cell_id: String,
    },
    CellMoved {
        cell_id: String,
    },
    CellTypeToggled {
        cell_id: String,
    },
    CellInputUpdated {
        cell_id: String,
    },
    CellStatusUpdated {
        cell_id: String,
    },
    CellOutputUpdated {
        cell_id: String,
    },
    /// A run was blocked because the notebook is untrusted; the window asks
    /// the user to trust it.  Changes nothing in the notebook.
    TrustRequired {
        cell_id: String,
        functions: Vec<String>,
    },
    /// The notebook's file path was set by opening or saving, or cleared
    /// (`None`) because its cells no longer match the file.
    PathChanged {
        path: Option<String>,
    },
    NotebookTrusted,
    NotebookReplaced,
    Undone,
    Redone,
    NoOp {
        reason: String,
    },
}

impl CommandEffect {
    /// The affected cell ID, if any.
    pub fn cell_id(&self) -> Option<&str> {
        match self {
            CommandEffect::CellAdded { cell_id }
            | CommandEffect::CellDeleted { cell_id }
            | CommandEffect::CellMoved { cell_id }
            | CommandEffect::CellTypeToggled { cell_id }
            | CommandEffect::CellInputUpdated { cell_id }
            | CommandEffect::CellStatusUpdated { cell_id }
            | CommandEffect::CellOutputUpdated { cell_id }
            | CommandEffect::TrustRequired { cell_id, .. } => Some(cell_id),
            _ => None,
        }
    }

    /// The effect type as a string for event payloads.
    pub fn effect_name(&self) -> &str {
        match self {
            CommandEffect::CellAdded { .. } => "cell_added",
            CommandEffect::CellDeleted { .. } => "cell_deleted",
            CommandEffect::CellMoved { .. } => "cell_moved",
            CommandEffect::CellTypeToggled { .. } => "cell_type_toggled",
            CommandEffect::CellInputUpdated { .. } => "cell_input_updated",
            CommandEffect::CellStatusUpdated { .. } => "cell_status_updated",
            CommandEffect::CellOutputUpdated { .. } => "cell_output_updated",
            CommandEffect::TrustRequired { .. } => "trust_required",
            CommandEffect::PathChanged { .. } => "path_changed",
            CommandEffect::NotebookTrusted => "notebook_trusted",
            CommandEffect::NotebookReplaced => "notebook_replaced",
            CommandEffect::Undone => "undone",
            CommandEffect::Redone => "redone",
            CommandEffect::NoOp { .. } => "no_op",
        }
    }
}

/// Receives effects as operations produce them, so a front end can show
/// each change while a long evaluation is still running.  Called with the
/// notebook still locked, right after the change: an implementation must not
/// lock the notebook again, and should only emit or queue.
pub trait EffectSink: Send + Sync {
    fn effect(&self, notebook_id: &str, notebook: &Notebook, effect: &CommandEffect);
}

/// A sink for callers with no window to update, such as batch runs.
pub struct NoEffects;

impl EffectSink for NoEffects {
    fn effect(&self, _notebook_id: &str, _notebook: &Notebook, _effect: &CommandEffect) {}
}
