use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::maxima::output::OutputEvent;
use crate::maxima::types::EvalResult;

use crate::commands::{CommandEffect, NotebookCommand};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellType {
    Code,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellStatus {
    Idle,
    Running,
    Success,
    Error,
    PendingApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellOutput {
    pub text_output: String,
    pub latex: Option<String>,
    pub plot_svg: Option<String>,
    pub error: Option<String>,
    pub is_error: bool,
    pub duration_ms: u64,
    pub output_label: Option<String>,
    pub execution_count: Option<u32>,
}

impl CellOutput {
    pub fn from_eval_result(result: &EvalResult, execution_count: u32) -> Self {
        CellOutput {
            text_output: result.text_output.clone(),
            latex: result.latex.clone(),
            plot_svg: result.plot_svg.clone(),
            error: result.error.clone(),
            is_error: result.is_error,
            duration_ms: result.duration_ms,
            output_label: result.output_label.clone(),
            execution_count: Some(execution_count),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    pub id: String,
    pub cell_type: CellType,
    pub input: String,
    pub output: Option<CellOutput>,
    pub status: CellStatus,
    pub raw_output: Vec<OutputEvent>,
    /// Whether the user has seen/approved this cell's content.
    /// Not serialized — always false on load.
    #[serde(skip)]
    pub trusted: bool,
    /// Dangerous functions detected when cell is in PendingApproval status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dangerous_functions: Option<Vec<String>>,
}

const MAX_UNDO: usize = 50;

/// Snapshot of the notebook structural state for undo/redo.
#[derive(Debug, Clone)]
struct NotebookSnapshot {
    cells: Vec<Cell>,
    execution_counter: u32,
}

pub struct Notebook {
    cells: Vec<Cell>,
    execution_counter: u32,
    /// Maps display count → real Maxima %oN label
    label_map: HashMap<u32, String>,
    undo_past: Vec<NotebookSnapshot>,
    undo_future: Vec<NotebookSnapshot>,
}

impl Notebook {
    pub fn new() -> Self {
        let initial_cell = Cell {
            id: new_cell_id(),
            cell_type: CellType::Code,
            input: String::new(),
            output: None,
            status: CellStatus::Idle,
            raw_output: Vec::new(),
            trusted: false,
            dangerous_functions: None,
        };
        Notebook {
            cells: vec![initial_cell],
            execution_counter: 0,
            label_map: HashMap::new(),
            undo_past: Vec::new(),
            undo_future: Vec::new(),
        }
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn get_cell(&self, id: &str) -> Option<&Cell> {
        self.cells.iter().find(|c| c.id == id)
    }

    pub fn get_cell_mut(&mut self, id: &str) -> Option<&mut Cell> {
        self.cells.iter_mut().find(|c| c.id == id)
    }

    pub fn add_cell(&mut self, cell_type: CellType, input: String, after_cell_id: Option<&str>, before_cell_id: Option<&str>) -> String {
        self.add_cell_with_id(new_cell_id(), cell_type, input, after_cell_id, before_cell_id)
    }

    /// Like `add_cell` but uses the provided ID instead of generating one.
    /// Used by sync to preserve frontend cell IDs.
    pub fn add_cell_with_id(
        &mut self,
        id: String,
        cell_type: CellType,
        input: String,
        after_cell_id: Option<&str>,
        before_cell_id: Option<&str>,
    ) -> String {
        let cell = Cell {
            id: id.clone(),
            cell_type,
            input,
            output: None,
            status: CellStatus::Idle,
            raw_output: Vec::new(),
            trusted: false,
            dangerous_functions: None,
        };

        if let Some(before_id) = before_cell_id {
            if let Some(pos) = self.cells.iter().position(|c| c.id == before_id) {
                self.cells.insert(pos, cell);
            } else {
                self.cells.push(cell);
            }
        } else if let Some(after_id) = after_cell_id {
            if let Some(pos) = self.cells.iter().position(|c| c.id == after_id) {
                self.cells.insert(pos + 1, cell);
            } else {
                self.cells.push(cell);
            }
        } else {
            self.cells.push(cell);
        }

        id
    }

    pub fn update_cell(&mut self, id: &str, input: Option<String>, cell_type: Option<CellType>) -> bool {
        if let Some(cell) = self.get_cell_mut(id) {
            if let Some(input) = input {
                cell.input = input;
            }
            if let Some(ct) = cell_type {
                cell.cell_type = ct;
            }
            true
        } else {
            false
        }
    }

    pub fn delete_cell(&mut self, id: &str) -> bool {
        if self.cells.len() <= 1 {
            return false;
        }
        let before = self.cells.len();
        self.cells.retain(|c| c.id != id);
        self.cells.len() < before
    }

    pub fn move_cell(&mut self, id: &str, direction: &str) -> bool {
        let pos = match self.cells.iter().position(|c| c.id == id) {
            Some(p) => p,
            None => return false,
        };

        match direction {
            "up" if pos > 0 => {
                self.cells.swap(pos, pos - 1);
                true
            }
            "down" if pos < self.cells.len() - 1 => {
                self.cells.swap(pos, pos + 1);
                true
            }
            _ => false,
        }
    }

    pub fn next_execution_count(&mut self) -> u32 {
        self.execution_counter += 1;
        self.execution_counter
    }

    pub fn label_map(&self) -> &HashMap<u32, String> {
        &self.label_map
    }

    pub fn record_label(&mut self, execution_count: u32, label: String) {
        self.label_map.insert(execution_count, label);
    }

    /// Get the previous output label for bare % resolution.
    pub fn previous_output_label(&self, cell_id: &str) -> Option<String> {
        let pos = self.cells.iter().position(|c| c.id == cell_id)?;
        // Walk backwards from the cell before this one
        for i in (0..pos).rev() {
            if let Some(ref output) = self.cells[i].output {
                if let Some(ref label) = output.output_label {
                    return Some(label.clone());
                }
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.execution_counter = 0;
        self.label_map.clear();
        self.undo_past.clear();
        self.undo_future.clear();
        self.cells.push(Cell {
            id: new_cell_id(),
            cell_type: CellType::Code,
            input: String::new(),
            output: None,
            status: CellStatus::Idle,
            raw_output: Vec::new(),
            trusted: false,
            dangerous_functions: None,
        });
    }

    /// Apply a command to the notebook, returning the effect describing what changed.
    pub fn apply(&mut self, cmd: NotebookCommand) -> Result<CommandEffect, String> {
        match cmd {
            NotebookCommand::AddCell {
                cell_type,
                input,
                after_cell_id,
                before_cell_id,
            } => {
                self.push_undo_snapshot();
                let id = self.add_cell(cell_type, input, after_cell_id.as_deref(), before_cell_id.as_deref());
                Ok(CommandEffect::CellAdded { cell_id: id })
            }

            NotebookCommand::DeleteCell { cell_id } => {
                if self.cells.len() <= 1 {
                    return Ok(CommandEffect::NoOp {
                        reason: "Cannot delete the last cell".into(),
                    });
                }
                if !self.cells.iter().any(|c| c.id == cell_id) {
                    return Err(format!("Cell '{}' not found", cell_id));
                }
                self.push_undo_snapshot();
                self.delete_cell(&cell_id);
                Ok(CommandEffect::CellDeleted { cell_id })
            }

            NotebookCommand::MoveCell {
                cell_id,
                direction,
            } => {
                // Pre-check if move is possible
                let pos = self
                    .cells
                    .iter()
                    .position(|c| c.id == cell_id)
                    .ok_or_else(|| format!("Cell '{}' not found", cell_id))?;
                let can_move = match direction.as_str() {
                    "up" => pos > 0,
                    "down" => pos < self.cells.len() - 1,
                    _ => false,
                };
                if !can_move {
                    return Ok(CommandEffect::NoOp {
                        reason: format!("Cannot move cell '{}' {}", cell_id, direction),
                    });
                }
                self.push_undo_snapshot();
                self.move_cell(&cell_id, &direction);
                Ok(CommandEffect::CellMoved { cell_id })
            }

            NotebookCommand::ToggleCellType { cell_id } => {
                if self.get_cell(&cell_id).is_none() {
                    return Err(format!("Cell '{}' not found", cell_id));
                }
                self.push_undo_snapshot();
                let cell = self.get_cell_mut(&cell_id).unwrap();
                cell.cell_type = match cell.cell_type {
                    CellType::Code => CellType::Markdown,
                    CellType::Markdown => CellType::Code,
                };
                cell.output = None;
                cell.status = CellStatus::Idle;
                cell.raw_output.clear();
                Ok(CommandEffect::CellTypeToggled { cell_id })
            }

            NotebookCommand::UpdateCellInput { cell_id, input, trusted } => {
                let unchanged = self
                    .get_cell(&cell_id)
                    .ok_or_else(|| format!("Cell '{}' not found", cell_id))
                    .map(|c| c.input == input)?;
                if unchanged {
                    return Ok(CommandEffect::NoOp {
                        reason: "Input unchanged".into(),
                    });
                }
                self.push_undo_snapshot();
                let cell = self.get_cell_mut(&cell_id).unwrap();
                cell.input = input;
                cell.trusted = trusted;
                Ok(CommandEffect::CellInputUpdated { cell_id })
            }

            NotebookCommand::SetCellStatus { cell_id, status } => {
                let cell = self
                    .get_cell_mut(&cell_id)
                    .ok_or_else(|| format!("Cell '{}' not found", cell_id))?;
                cell.status = status;
                // Not undoable — no snapshot pushed
                Ok(CommandEffect::CellStatusUpdated { cell_id })
            }

            NotebookCommand::SetCellOutput {
                cell_id,
                output,
                raw_output,
            } => {
                let cell = self
                    .get_cell_mut(&cell_id)
                    .ok_or_else(|| format!("Cell '{}' not found", cell_id))?;
                cell.status = if output.is_error {
                    CellStatus::Error
                } else {
                    CellStatus::Success
                };
                cell.output = Some(output);
                cell.raw_output = raw_output;
                // Not undoable — no snapshot pushed
                Ok(CommandEffect::CellOutputUpdated { cell_id })
            }

            NotebookCommand::NewNotebook => {
                self.push_undo_snapshot();
                self.cells.clear();
                self.execution_counter = 0;
                self.label_map.clear();
                self.cells.push(Cell {
                    id: new_cell_id(),
                    cell_type: CellType::Code,
                    input: String::new(),
                    output: None,
                    status: CellStatus::Idle,
                    raw_output: Vec::new(),
                    trusted: false,
                    dangerous_functions: None,
                });
                Ok(CommandEffect::NotebookReplaced)
            }

            NotebookCommand::LoadCells { cells } => {
                self.push_undo_snapshot();
                self.load_cells_from_list(cells);
                Ok(CommandEffect::NotebookReplaced)
            }

            NotebookCommand::SetCellPendingApproval {
                cell_id,
                dangerous_functions,
            } => {
                let cell = self
                    .get_cell_mut(&cell_id)
                    .ok_or_else(|| format!("Cell '{}' not found", cell_id))?;
                cell.status = CellStatus::PendingApproval;
                cell.dangerous_functions = Some(dangerous_functions);
                Ok(CommandEffect::CellPendingApproval { cell_id })
            }

            NotebookCommand::ApproveCellExecution { cell_id } => {
                let cell = self
                    .get_cell_mut(&cell_id)
                    .ok_or_else(|| format!("Cell '{}' not found", cell_id))?;
                cell.status = CellStatus::Idle;
                cell.trusted = true;
                cell.dangerous_functions = None;
                Ok(CommandEffect::CellApprovalCleared { cell_id })
            }

            NotebookCommand::AbortCellExecution { cell_id } => {
                let cell = self
                    .get_cell_mut(&cell_id)
                    .ok_or_else(|| format!("Cell '{}' not found", cell_id))?;
                cell.status = CellStatus::Idle;
                cell.dangerous_functions = None;
                Ok(CommandEffect::CellApprovalCleared { cell_id })
            }

            NotebookCommand::Undo => {
                if let Some(snapshot) = self.undo_past.pop() {
                    let current = NotebookSnapshot {
                        cells: self.cells.clone(),
                        execution_counter: self.execution_counter,
                    };
                    self.undo_future.push(current);
                    self.cells = snapshot.cells;
                    self.execution_counter = snapshot.execution_counter;
                    Ok(CommandEffect::Undone)
                } else {
                    Ok(CommandEffect::NoOp {
                        reason: "Nothing to undo".into(),
                    })
                }
            }

            NotebookCommand::Redo => {
                if let Some(snapshot) = self.undo_future.pop() {
                    let current = NotebookSnapshot {
                        cells: self.cells.clone(),
                        execution_counter: self.execution_counter,
                    };
                    self.undo_past.push(current);
                    self.cells = snapshot.cells;
                    self.execution_counter = snapshot.execution_counter;
                    Ok(CommandEffect::Redone)
                } else {
                    Ok(CommandEffect::NoOp {
                        reason: "Nothing to redo".into(),
                    })
                }
            }
        }
    }

    /// Push an undo snapshot of the current state. Clears the redo stack.
    fn push_undo_snapshot(&mut self) {
        self.undo_past.push(NotebookSnapshot {
            cells: self.cells.clone(),
            execution_counter: self.execution_counter,
        });
        if self.undo_past.len() > MAX_UNDO {
            self.undo_past.remove(0);
        }
        self.undo_future.clear();
    }

    /// Replace cells from a list of (id, cell_type, input) tuples.
    /// Used by LoadCells command.
    fn load_cells_from_list(&mut self, incoming: Vec<(String, CellType, String)>) {
        self.cells.clear();
        self.execution_counter = 0;
        self.label_map.clear();
        for (id, cell_type, input) in incoming {
            self.cells.push(Cell {
                id,
                cell_type,
                input,
                output: None,
                status: CellStatus::Idle,
                raw_output: Vec::new(),
                trusted: false,
                dangerous_functions: None,
            });
        }
        if self.cells.is_empty() {
            self.cells.push(Cell {
                id: new_cell_id(),
                cell_type: CellType::Code,
                input: String::new(),
                output: None,
                status: CellStatus::Idle,
                raw_output: Vec::new(),
                trusted: false,
                dangerous_functions: None,
            });
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.undo_future.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{CommandEffect, NotebookCommand};

    fn nb() -> Notebook {
        Notebook::new()
    }

    fn first_cell_id(nb: &Notebook) -> String {
        nb.cells()[0].id.clone()
    }

    #[test]
    fn add_cell_basic() {
        let mut n = nb();
        let effect = n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "x: 42;".into(),
            after_cell_id: None,
            before_cell_id: None,
        }).unwrap();
        assert!(matches!(effect, CommandEffect::CellAdded { .. }));
        assert_eq!(n.cells().len(), 2);
        assert_eq!(n.cells()[1].input, "x: 42;");
    }

    #[test]
    fn add_cell_after() {
        let mut n = nb();
        let first = first_cell_id(&n);
        let effect = n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Markdown,
            input: "# Title".into(),
            after_cell_id: Some(first.clone()),
            before_cell_id: None,
        }).unwrap();
        let new_id = match effect {
            CommandEffect::CellAdded { ref cell_id } => cell_id.clone(),
            _ => panic!("Expected CellAdded"),
        };
        // New cell should be at index 1, right after the first cell
        assert_eq!(n.cells()[1].id, new_id);
        assert_eq!(n.cells()[1].cell_type, CellType::Markdown);
    }

    #[test]
    fn add_cell_before() {
        let mut n = nb();
        let first = first_cell_id(&n);
        let effect = n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "before first".into(),
            after_cell_id: None,
            before_cell_id: Some(first.clone()),
        }).unwrap();
        let new_id = match effect {
            CommandEffect::CellAdded { ref cell_id } => cell_id.clone(),
            _ => panic!("Expected CellAdded"),
        };
        // New cell should be at index 0, before the original first cell
        assert_eq!(n.cells()[0].id, new_id);
        assert_eq!(n.cells()[0].input, "before first");
        assert_eq!(n.cells()[1].id, first);
    }

    #[test]
    fn delete_cell() {
        let mut n = nb();
        n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "".into(),
            after_cell_id: None,
            before_cell_id: None,
        }).unwrap();
        assert_eq!(n.cells().len(), 2);
        let id = n.cells()[1].id.clone();
        let effect = n.apply(NotebookCommand::DeleteCell { cell_id: id }).unwrap();
        assert!(matches!(effect, CommandEffect::CellDeleted { .. }));
        assert_eq!(n.cells().len(), 1);
    }

    #[test]
    fn delete_last_cell_is_noop() {
        let mut n = nb();
        assert_eq!(n.cells().len(), 1);
        let id = first_cell_id(&n);
        let effect = n.apply(NotebookCommand::DeleteCell { cell_id: id }).unwrap();
        assert!(matches!(effect, CommandEffect::NoOp { .. }));
        assert_eq!(n.cells().len(), 1);
    }

    #[test]
    fn move_cell() {
        let mut n = nb();
        let first = first_cell_id(&n);
        let effect = n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "second".into(),
            after_cell_id: None,
            before_cell_id: None,
        }).unwrap();
        let second = match effect {
            CommandEffect::CellAdded { cell_id } => cell_id,
            _ => panic!(),
        };
        // Move second cell up
        let effect = n.apply(NotebookCommand::MoveCell {
            cell_id: second.clone(),
            direction: "up".into(),
        }).unwrap();
        assert!(matches!(effect, CommandEffect::CellMoved { .. }));
        assert_eq!(n.cells()[0].id, second);
        assert_eq!(n.cells()[1].id, first);
    }

    #[test]
    fn move_cell_boundary_is_noop() {
        let mut n = nb();
        let first = first_cell_id(&n);
        let effect = n.apply(NotebookCommand::MoveCell {
            cell_id: first,
            direction: "up".into(),
        }).unwrap();
        assert!(matches!(effect, CommandEffect::NoOp { .. }));
    }

    #[test]
    fn toggle_cell_type() {
        let mut n = nb();
        let id = first_cell_id(&n);
        assert_eq!(n.cells()[0].cell_type, CellType::Code);
        n.apply(NotebookCommand::ToggleCellType { cell_id: id.clone() }).unwrap();
        assert_eq!(n.cells()[0].cell_type, CellType::Markdown);
        n.apply(NotebookCommand::ToggleCellType { cell_id: id }).unwrap();
        assert_eq!(n.cells()[0].cell_type, CellType::Code);
    }

    #[test]
    fn update_cell_input() {
        let mut n = nb();
        let id = first_cell_id(&n);
        let effect = n.apply(NotebookCommand::UpdateCellInput {
            cell_id: id.clone(),
            input: "y: 10;".into(),
            trusted: true,
        }).unwrap();
        assert!(matches!(effect, CommandEffect::CellInputUpdated { .. }));
        assert_eq!(n.cells()[0].input, "y: 10;");
    }

    #[test]
    fn update_cell_input_unchanged_is_noop() {
        let mut n = nb();
        let id = first_cell_id(&n);
        let effect = n.apply(NotebookCommand::UpdateCellInput {
            cell_id: id,
            input: "".into(),
            trusted: true,
        }).unwrap();
        assert!(matches!(effect, CommandEffect::NoOp { .. }));
    }

    #[test]
    fn set_cell_status() {
        let mut n = nb();
        let id = first_cell_id(&n);
        n.apply(NotebookCommand::SetCellStatus {
            cell_id: id.clone(),
            status: CellStatus::Running,
        }).unwrap();
        assert_eq!(n.cells()[0].status, CellStatus::Running);
        // Not undoable: no undo snapshot
        assert!(!n.can_undo());
    }

    #[test]
    fn new_notebook() {
        let mut n = nb();
        // Add some cells
        n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "x".into(),
            after_cell_id: None,
            before_cell_id: None,
        }).unwrap();
        assert_eq!(n.cells().len(), 2);
        n.apply(NotebookCommand::NewNotebook).unwrap();
        assert_eq!(n.cells().len(), 1);
        assert_eq!(n.cells()[0].input, "");
    }

    #[test]
    fn load_cells() {
        let mut n = nb();
        let cells = vec![
            ("a".into(), CellType::Markdown, "# Hello".into()),
            ("b".into(), CellType::Code, "x: 1;".into()),
        ];
        n.apply(NotebookCommand::LoadCells { cells }).unwrap();
        assert_eq!(n.cells().len(), 2);
        assert_eq!(n.cells()[0].id, "a");
        assert_eq!(n.cells()[1].input, "x: 1;");
    }

    #[test]
    fn undo_redo_add_cell() {
        let mut n = nb();
        let original_id = first_cell_id(&n);
        // Add a cell
        n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "added".into(),
            after_cell_id: None,
            before_cell_id: None,
        }).unwrap();
        assert_eq!(n.cells().len(), 2);
        assert!(n.can_undo());

        // Undo
        let effect = n.apply(NotebookCommand::Undo).unwrap();
        assert!(matches!(effect, CommandEffect::Undone));
        assert_eq!(n.cells().len(), 1);
        assert_eq!(n.cells()[0].id, original_id);
        assert!(n.can_redo());

        // Redo
        let effect = n.apply(NotebookCommand::Redo).unwrap();
        assert!(matches!(effect, CommandEffect::Redone));
        assert_eq!(n.cells().len(), 2);
        assert_eq!(n.cells()[1].input, "added");
    }

    #[test]
    fn undo_nothing_is_noop() {
        let mut n = nb();
        let effect = n.apply(NotebookCommand::Undo).unwrap();
        assert!(matches!(effect, CommandEffect::NoOp { .. }));
    }

    #[test]
    fn redo_nothing_is_noop() {
        let mut n = nb();
        let effect = n.apply(NotebookCommand::Redo).unwrap();
        assert!(matches!(effect, CommandEffect::NoOp { .. }));
    }

    #[test]
    fn undo_clears_redo_on_new_command() {
        let mut n = nb();
        // Add, then undo to create redo state
        n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "".into(),
            after_cell_id: None,
            before_cell_id: None,
        }).unwrap();
        n.apply(NotebookCommand::Undo).unwrap();
        assert!(n.can_redo());

        // New command should clear redo
        n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "new".into(),
            after_cell_id: None,
            before_cell_id: None,
        }).unwrap();
        assert!(!n.can_redo());
    }

    #[test]
    fn max_undo_depth() {
        let mut n = nb();
        for i in 0..60 {
            n.apply(NotebookCommand::UpdateCellInput {
                cell_id: first_cell_id(&n),
                input: format!("v{}", i),
                trusted: true,
            }).unwrap();
        }
        // Should cap at MAX_UNDO
        assert_eq!(n.undo_past.len(), MAX_UNDO);
    }

    #[test]
    fn delete_cell_not_found() {
        let mut n = nb();
        // Add a second cell so "last cell" guard doesn't trigger first
        n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "".into(),
            after_cell_id: None,
            before_cell_id: None,
        }).unwrap();
        let result = n.apply(NotebookCommand::DeleteCell {
            cell_id: "nonexistent".into(),
        });
        assert!(result.is_err());
    }
}

fn new_cell_id() -> String {
    nanoid::nanoid!()
}
