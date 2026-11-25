use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::maxima::labels::LabelContext;
use crate::maxima::output::OutputEvent;
use crate::maxima::types::{ErrorInfo, EvalResult};
use crate::notebooks::format::LoadedCell;

use crate::commands::{CommandEffect, NotebookCommand};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellType {
    Code,
    Markdown,
    /// Text kept from a file (nbformat raw cells): shown read-only, never
    /// run, written back unchanged.
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellStatus {
    Idle,
    Running,
    Success,
    Error,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CellOutput {
    pub text_output: String,
    pub latex: Option<String>,
    pub plot_svg: Option<String>,
    pub plot_data: Option<String>,
    pub image_png: Option<String>,
    pub error: Option<String>,
    pub is_error: bool,
    pub duration_ms: u64,
    pub output_label: Option<String>,
    pub execution_count: Option<u32>,
    /// Maxima's one-line form of the final value, as `string(%)` gives it.
    /// Written as the result's `text/plain` copy in files.
    #[serde(default)]
    pub text_plain: Option<String>,
    /// A friendly explanation of the error, for the window.
    #[serde(default)]
    pub error_info: Option<ErrorInfo>,
    /// Maxima's error message in the user's language, for the window.
    /// Not written to files.
    #[serde(default)]
    pub error_translated: Option<String>,
    /// Read from a file: the output of a session that no longer exists.
    #[serde(default)]
    pub stale: bool,
    /// Outputs read from a file that Aximar doesn't show, such as other
    /// output types or bundles of unknown MIME types.  Written back
    /// unchanged; never sent to the window.
    #[serde(skip)]
    pub other_outputs: Vec<Value>,
}

impl CellOutput {
    pub fn from_eval_result(result: &EvalResult, execution_count: u32) -> Self {
        CellOutput {
            text_output: result.text_output.clone(),
            latex: result.latex.clone(),
            plot_svg: result.plot_svg.clone(),
            plot_data: result.plot_data.clone(),
            image_png: result.image_png.clone(),
            error: result.error.clone(),
            is_error: result.is_error,
            duration_ms: result.duration_ms,
            output_label: result.output_label.clone(),
            execution_count: Some(execution_count),
            text_plain: result.text_plain.clone(),
            error_info: result.error_info.clone(),
            error_translated: result.error_translated.clone(),
            stale: false,
            other_outputs: Vec::new(),
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
    /// Cell metadata from the file, kept and written back unchanged.
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub metadata: Map<String, Value>,
    /// Other fields of the cell in the file, such as Jupyter's
    /// `attachments`, kept and written back unchanged.
    #[serde(skip)]
    pub extra: Map<String, Value>,
}

const MAX_UNDO: usize = 50;

/// Snapshot of the notebook structural state for undo/redo.
#[derive(Debug, Clone)]
struct NotebookSnapshot {
    cells: Vec<Cell>,
    execution_counter: u32,
    metadata: Map<String, Value>,
}

pub struct Notebook {
    cells: Vec<Cell>,
    /// Notebook metadata from the file, kept and written back.
    metadata: Map<String, Value>,
    execution_counter: u32,
    /// Maps display count → real Maxima %oN label
    label_map: HashMap<u32, String>,
    /// Maps display count → the cell whose run recorded that label.  After a
    /// restart a cell can still show an old count that a new run reuses.
    label_cells: HashMap<u32, String>,
    /// Cells a client keeps itself (the VS Code extension), which run here
    /// by ID: each cell's latest run in this session.
    client_runs: HashMap<String, u32>,
    undo_past: Vec<NotebookSnapshot>,
    undo_future: Vec<NotebookSnapshot>,
    /// Whether the notebook is trusted for execution of dangerous functions.
    /// Session-only — not persisted. `true` for new notebooks, `false` when loaded from disk.
    trusted: bool,
}

impl Default for Notebook {
    fn default() -> Self {
        Self::new()
    }
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
            metadata: Map::new(),
            extra: Map::new(),
        };
        Notebook {
            cells: vec![initial_cell],
            metadata: Map::new(),
            execution_counter: 0,
            label_map: HashMap::new(),
            label_cells: HashMap::new(),
            client_runs: HashMap::new(),
            undo_past: Vec::new(),
            undo_future: Vec::new(),
            trusted: true,
        }
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Notebook metadata, as read from the file.
    pub fn metadata(&self) -> &Map<String, Value> {
        &self.metadata
    }

    pub fn get_cell(&self, id: &str) -> Option<&Cell> {
        self.cells.iter().find(|c| c.id == id)
    }

    pub fn get_cell_mut(&mut self, id: &str) -> Option<&mut Cell> {
        self.cells.iter_mut().find(|c| c.id == id)
    }

    pub fn trusted(&self) -> bool {
        self.trusted
    }

    pub fn set_trusted(&mut self, trusted: bool) {
        self.trusted = trusted;
    }

    pub fn add_cell(
        &mut self,
        cell_type: CellType,
        input: String,
        after_cell_id: Option<&str>,
        before_cell_id: Option<&str>,
    ) -> String {
        self.add_cell_with_id(
            new_cell_id(),
            cell_type,
            input,
            after_cell_id,
            before_cell_id,
        )
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
            metadata: Map::new(),
            extra: Map::new(),
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

    pub fn update_cell(
        &mut self,
        id: &str,
        input: Option<String>,
        cell_type: Option<CellType>,
    ) -> bool {
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

    pub fn record_label(&mut self, execution_count: u32, label: String, cell_id: &str) {
        self.label_map.insert(execution_count, label);
        self.label_cells
            .insert(execution_count, cell_id.to_string());
    }

    /// Start numbering afresh for a new Maxima process: forget every
    /// recorded label and count executions from 1 again, as Jupyter does on
    /// a kernel restart.  The new process reuses the old label numbers for
    /// unrelated values, and running the cells again in order gives them the
    /// same `In[n]`/`Out[n]` as before, so references like `%o4` keep working.
    pub fn start_new_session(&mut self) {
        self.label_map.clear();
        self.label_cells.clear();
        self.client_runs.clear();
        self.execution_counter = 0;
    }

    /// Labels for preparing a cell's input: a bare `%` at its start means
    /// the output of the nearest cell above that has one.
    pub fn cell_label_context(&self, cell_id: &str) -> LabelContext {
        LabelContext {
            label_map: self.label_map.clone(),
            previous_output_label: self.previous_output_label(cell_id),
            bare_percent: true,
        }
    }

    /// Labels for preparing an expression, which has no cell above it.
    pub fn expression_label_context(&self) -> LabelContext {
        LabelContext {
            label_map: self.label_map.clone(),
            previous_output_label: None,
            bare_percent: false,
        }
    }

    /// The previous output label for a bare `%`: the label of the nearest
    /// cell above whose output was produced in this session.  Going through
    /// the label map skips cells that errored and outputs left from an
    /// earlier session, even when a new run has reused their count.
    pub fn previous_output_label(&self, cell_id: &str) -> Option<String> {
        let pos = self.cells.iter().position(|c| c.id == cell_id)?;
        // Walk backwards from the cell before this one
        self.cells[..pos].iter().rev().find_map(|cell| {
            let count = cell.output.as_ref()?.execution_count?;
            self.output_label_of(&cell.id, count)
        })
    }

    /// The label a cell's run recorded, if that run is the one with this
    /// count in this session.
    fn output_label_of(&self, cell_id: &str, count: u32) -> Option<String> {
        if self.label_cells.get(&count).map(String::as_str) != Some(cell_id) {
            return None;
        }
        self.label_map.get(&count).cloned()
    }

    /// Labels for a cell a client keeps itself, given the IDs of the code
    /// cells above it in order.  The same rules as for this notebook's own
    /// cells: a bare `%` means the nearest cell above whose latest run in
    /// this session produced an output.
    pub fn client_cell_label_context(&self, cells_above: &[String]) -> LabelContext {
        let previous = cells_above.iter().rev().find_map(|id| {
            let count = *self.client_runs.get(id)?;
            self.output_label_of(id, count)
        });
        LabelContext {
            label_map: self.label_map.clone(),
            previous_output_label: previous,
            bare_percent: true,
        }
    }

    /// Record a run of a client's cell: its count, and its label when it
    /// produced an output.  A run without one hides the cell's earlier
    /// output from a bare `%`, as an error output does in this notebook.
    pub fn record_client_run(&mut self, cell_id: &str, count: u32, label: Option<String>) {
        self.client_runs.insert(cell_id.to_string(), count);
        if let Some(label) = label {
            self.record_label(count, label, cell_id);
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.execution_counter = 0;
        self.label_map.clear();
        self.label_cells.clear();
        self.client_runs.clear();
        self.undo_past.clear();
        self.undo_future.clear();
        self.cells.push(Cell {
            id: new_cell_id(),
            cell_type: CellType::Code,
            input: String::new(),
            output: None,
            status: CellStatus::Idle,
            raw_output: Vec::new(),
            metadata: Map::new(),
            extra: Map::new(),
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
                let id = self.add_cell(
                    cell_type,
                    input,
                    after_cell_id.as_deref(),
                    before_cell_id.as_deref(),
                );
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

            NotebookCommand::MoveCell { cell_id, direction } => {
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
                    CellType::Markdown | CellType::Raw => CellType::Code,
                };
                cell.output = None;
                cell.status = CellStatus::Idle;
                cell.raw_output.clear();
                Ok(CommandEffect::CellTypeToggled { cell_id })
            }

            NotebookCommand::UpdateCellInput { cell_id, input } => {
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
                cell.output = Some(*output);
                cell.raw_output = raw_output;
                // Not undoable — no snapshot pushed
                Ok(CommandEffect::CellOutputUpdated { cell_id })
            }

            NotebookCommand::TrustNotebook { trusted } => {
                self.trusted = trusted;
                Ok(CommandEffect::NotebookTrusted)
            }

            NotebookCommand::NewNotebook => {
                self.push_undo_snapshot();
                self.cells.clear();
                self.metadata = Map::new();
                self.execution_counter = 0;
                self.label_map.clear();
                self.label_cells.clear();
                self.client_runs.clear();
                self.trusted = true;
                self.cells.push(Cell {
                    id: new_cell_id(),
                    cell_type: CellType::Code,
                    input: String::new(),
                    output: None,
                    status: CellStatus::Idle,
                    raw_output: Vec::new(),
                    metadata: Map::new(),
                    extra: Map::new(),
                });
                Ok(CommandEffect::NotebookReplaced)
            }

            NotebookCommand::LoadCells { cells } => {
                self.push_undo_snapshot();
                self.metadata = Map::new();
                let cells = cells
                    .into_iter()
                    .map(|(id, cell_type, input, output)| LoadedCell {
                        id,
                        cell_type,
                        input,
                        metadata: Map::new(),
                        extra: Map::new(),
                        output,
                    })
                    .collect();
                self.load_cells_from_list(cells);
                Ok(CommandEffect::NotebookReplaced)
            }

            NotebookCommand::LoadNotebook { file } => {
                let file = *file;
                self.push_undo_snapshot();
                self.metadata = file.metadata;
                self.load_cells_from_list(file.cells);
                Ok(CommandEffect::NotebookReplaced)
            }

            NotebookCommand::Undo => {
                if let Some(snapshot) = self.undo_past.pop() {
                    let current = NotebookSnapshot {
                        cells: self.cells.clone(),
                        execution_counter: self.execution_counter,
                        metadata: self.metadata.clone(),
                    };
                    self.undo_future.push(current);
                    self.cells = snapshot.cells;
                    self.execution_counter = snapshot.execution_counter;
                    self.metadata = snapshot.metadata;
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
                        metadata: self.metadata.clone(),
                    };
                    self.undo_past.push(current);
                    self.cells = snapshot.cells;
                    self.execution_counter = snapshot.execution_counter;
                    self.metadata = snapshot.metadata;
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
            metadata: self.metadata.clone(),
        });
        if self.undo_past.len() > MAX_UNDO {
            self.undo_past.remove(0);
        }
        self.undo_future.clear();
    }

    /// Replace the cells with ones read from a file.
    fn load_cells_from_list(&mut self, incoming: Vec<LoadedCell>) {
        self.cells.clear();
        self.execution_counter = 0;
        self.label_map.clear();
        self.label_cells.clear();
        self.client_runs.clear();
        self.trusted = false;
        // Numbering starts at 1 whatever the loaded outputs show: they came
        // from a session that no longer exists, and record no labels (#8)
        for LoadedCell {
            id,
            cell_type,
            input,
            metadata,
            extra,
            output,
        } in incoming
        {
            let status = match &output {
                Some(out) if out.is_error => CellStatus::Error,
                Some(_) => CellStatus::Success,
                None => CellStatus::Idle,
            };
            self.cells.push(Cell {
                id,
                cell_type,
                input,
                output,
                status,
                raw_output: Vec::new(),
                metadata,
                extra,
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
                metadata: Map::new(),
                extra: Map::new(),
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

pub(crate) fn new_cell_id() -> String {
    aximar_notebook_format::new_id()
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
        let effect = n
            .apply(NotebookCommand::AddCell {
                cell_type: CellType::Code,
                input: "x: 42;".into(),
                after_cell_id: None,
                before_cell_id: None,
            })
            .unwrap();
        assert!(matches!(effect, CommandEffect::CellAdded { .. }));
        assert_eq!(n.cells().len(), 2);
        assert_eq!(n.cells()[1].input, "x: 42;");
    }

    #[test]
    fn add_cell_after() {
        let mut n = nb();
        let first = first_cell_id(&n);
        let effect = n
            .apply(NotebookCommand::AddCell {
                cell_type: CellType::Markdown,
                input: "# Title".into(),
                after_cell_id: Some(first.clone()),
                before_cell_id: None,
            })
            .unwrap();
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
        let effect = n
            .apply(NotebookCommand::AddCell {
                cell_type: CellType::Code,
                input: "before first".into(),
                after_cell_id: None,
                before_cell_id: Some(first.clone()),
            })
            .unwrap();
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
        })
        .unwrap();
        assert_eq!(n.cells().len(), 2);
        let id = n.cells()[1].id.clone();
        let effect = n
            .apply(NotebookCommand::DeleteCell { cell_id: id })
            .unwrap();
        assert!(matches!(effect, CommandEffect::CellDeleted { .. }));
        assert_eq!(n.cells().len(), 1);
    }

    #[test]
    fn delete_last_cell_is_noop() {
        let mut n = nb();
        assert_eq!(n.cells().len(), 1);
        let id = first_cell_id(&n);
        let effect = n
            .apply(NotebookCommand::DeleteCell { cell_id: id })
            .unwrap();
        assert!(matches!(effect, CommandEffect::NoOp { .. }));
        assert_eq!(n.cells().len(), 1);
    }

    #[test]
    fn move_cell() {
        let mut n = nb();
        let first = first_cell_id(&n);
        let effect = n
            .apply(NotebookCommand::AddCell {
                cell_type: CellType::Code,
                input: "second".into(),
                after_cell_id: None,
                before_cell_id: None,
            })
            .unwrap();
        let second = match effect {
            CommandEffect::CellAdded { cell_id } => cell_id,
            _ => panic!(),
        };
        // Move second cell up
        let effect = n
            .apply(NotebookCommand::MoveCell {
                cell_id: second.clone(),
                direction: "up".into(),
            })
            .unwrap();
        assert!(matches!(effect, CommandEffect::CellMoved { .. }));
        assert_eq!(n.cells()[0].id, second);
        assert_eq!(n.cells()[1].id, first);
    }

    #[test]
    fn move_cell_boundary_is_noop() {
        let mut n = nb();
        let first = first_cell_id(&n);
        let effect = n
            .apply(NotebookCommand::MoveCell {
                cell_id: first,
                direction: "up".into(),
            })
            .unwrap();
        assert!(matches!(effect, CommandEffect::NoOp { .. }));
    }

    #[test]
    fn toggle_cell_type() {
        let mut n = nb();
        let id = first_cell_id(&n);
        assert_eq!(n.cells()[0].cell_type, CellType::Code);
        n.apply(NotebookCommand::ToggleCellType {
            cell_id: id.clone(),
        })
        .unwrap();
        assert_eq!(n.cells()[0].cell_type, CellType::Markdown);
        n.apply(NotebookCommand::ToggleCellType { cell_id: id })
            .unwrap();
        assert_eq!(n.cells()[0].cell_type, CellType::Code);
    }

    #[test]
    fn update_cell_input() {
        let mut n = nb();
        let id = first_cell_id(&n);
        let effect = n
            .apply(NotebookCommand::UpdateCellInput {
                cell_id: id.clone(),
                input: "y: 10;".into(),
            })
            .unwrap();
        assert!(matches!(effect, CommandEffect::CellInputUpdated { .. }));
        assert_eq!(n.cells()[0].input, "y: 10;");
    }

    #[test]
    fn update_cell_input_unchanged_is_noop() {
        let mut n = nb();
        let id = first_cell_id(&n);
        let effect = n
            .apply(NotebookCommand::UpdateCellInput {
                cell_id: id,
                input: "".into(),
            })
            .unwrap();
        assert!(matches!(effect, CommandEffect::NoOp { .. }));
    }

    #[test]
    fn set_cell_status() {
        let mut n = nb();
        let id = first_cell_id(&n);
        n.apply(NotebookCommand::SetCellStatus {
            cell_id: id.clone(),
            status: CellStatus::Running,
        })
        .unwrap();
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
        })
        .unwrap();
        assert_eq!(n.cells().len(), 2);
        n.apply(NotebookCommand::NewNotebook).unwrap();
        assert_eq!(n.cells().len(), 1);
        assert_eq!(n.cells()[0].input, "");
    }

    #[test]
    fn load_cells() {
        let mut n = nb();
        let cells = vec![
            ("a".into(), CellType::Markdown, "# Hello".into(), None),
            ("b".into(), CellType::Code, "x: 1;".into(), None),
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
        })
        .unwrap();
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
        })
        .unwrap();
        n.apply(NotebookCommand::Undo).unwrap();
        assert!(n.can_redo());

        // New command should clear redo
        n.apply(NotebookCommand::AddCell {
            cell_type: CellType::Code,
            input: "new".into(),
            after_cell_id: None,
            before_cell_id: None,
        })
        .unwrap();
        assert!(!n.can_redo());
    }

    #[test]
    fn max_undo_depth() {
        let mut n = nb();
        for i in 0..60 {
            n.apply(NotebookCommand::UpdateCellInput {
                cell_id: first_cell_id(&n),
                input: format!("v{}", i),
            })
            .unwrap();
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
        })
        .unwrap();
        let result = n.apply(NotebookCommand::DeleteCell {
            cell_id: "nonexistent".into(),
        });
        assert!(result.is_err());
    }

    /// Give a cell an output as `evaluate_cell` would, recording its label
    /// when the evaluation succeeded.
    fn run(n: &mut Notebook, cell_id: &str, label: Option<&str>) {
        let count = n.next_execution_count();
        let result = EvalResult {
            output_label: label.map(String::from),
            is_error: label.is_none(),
            ..Default::default()
        };
        if let Some(label) = label {
            n.record_label(count, label.to_string(), cell_id);
        }
        n.apply(NotebookCommand::SetCellOutput {
            cell_id: cell_id.to_string(),
            output: Box::new(CellOutput::from_eval_result(&result, count)),
            raw_output: Vec::new(),
        })
        .unwrap();
    }

    fn add_code_cell(n: &mut Notebook) -> String {
        match n
            .apply(NotebookCommand::AddCell {
                cell_type: CellType::Code,
                input: String::new(),
                after_cell_id: None,
                before_cell_id: None,
            })
            .unwrap()
        {
            CommandEffect::CellAdded { cell_id } => cell_id,
            other => panic!("unexpected effect {other:?}"),
        }
    }

    #[test]
    fn previous_output_label_skips_cells_without_a_label() {
        let mut n = nb();
        let first = first_cell_id(&n);
        let failed = add_code_cell(&mut n);
        let current = add_code_cell(&mut n);
        run(&mut n, &first, Some("%o196"));
        run(&mut n, &failed, None);
        assert_eq!(n.previous_output_label(&current), Some("%o196".to_string()));
    }

    #[test]
    fn new_session_forgets_labels_and_counts_from_one() {
        let mut n = nb();
        let first = first_cell_id(&n);
        let current = add_code_cell(&mut n);
        run(&mut n, &first, Some("%o196"));
        n.start_new_session();
        assert!(n.label_map().is_empty());
        assert_eq!(n.previous_output_label(&current), None);
        assert_eq!(n.next_execution_count(), 1);
    }

    #[test]
    fn stale_output_does_not_match_a_reused_count() {
        // Before the restart: a ran as [1], b as [2]
        let mut n = nb();
        let a = first_cell_id(&n);
        let b = add_code_cell(&mut n);
        let c = add_code_cell(&mut n);
        run(&mut n, &a, Some("%o196"));
        run(&mut n, &b, Some("%o200"));
        n.start_new_session();
        // After it, b runs first and becomes [1]; a still shows the old [1]
        run(&mut n, &b, Some("%o196"));
        assert_eq!(n.previous_output_label(&c), Some("%o196".to_string()));
        // For b, the cell above is a, whose [1] belongs to the old session
        assert_eq!(n.previous_output_label(&b), None);
    }

    #[test]
    fn metadata_and_raw_cells_survive_undo_and_redo() {
        let text = r#"{"nbformat": 4, "nbformat_minor": 5, "metadata": {"jupytext": {"formats": "ipynb"}}, "cells": [
            {"id": "r", "cell_type": "raw", "source": "kept", "metadata": {"format": "text/plain"}},
            {"id": "c", "cell_type": "code", "source": "1;", "metadata": {"tags": ["t"]}, "execution_count": null, "outputs": []}]}"#;
        let file = crate::notebooks::format::read(text).unwrap();
        let mut n = nb();
        n.apply(NotebookCommand::LoadNotebook { file: file.into() })
            .unwrap();
        let check = |n: &Notebook| {
            assert_eq!(n.metadata()["jupytext"]["formats"], "ipynb");
            assert_eq!(n.cells()[0].cell_type, CellType::Raw);
            assert_eq!(n.cells()[0].metadata["format"], "text/plain");
            assert_eq!(n.cells()[1].metadata["tags"][0], "t");
        };
        check(&n);
        // An undoable edit, undone and redone
        n.apply(NotebookCommand::UpdateCellInput {
            cell_id: "c".into(),
            input: "2;".into(),
        })
        .unwrap();
        n.apply(NotebookCommand::Undo).unwrap();
        check(&n);
        n.apply(NotebookCommand::Redo).unwrap();
        check(&n);
        // Undoing the load itself restores the empty notebook's metadata
        n.apply(NotebookCommand::Undo).unwrap();
        n.apply(NotebookCommand::Undo).unwrap();
        assert!(n.metadata().is_empty());
        n.apply(NotebookCommand::Redo).unwrap();
        check(&n);
    }

    #[test]
    fn toggling_a_raw_cell_makes_it_code() {
        let text = r#"{"nbformat": 4, "nbformat_minor": 5, "metadata": {}, "cells": [
            {"id": "r", "cell_type": "raw", "source": "x;", "metadata": {}}]}"#;
        let mut n = nb();
        n.apply(NotebookCommand::LoadNotebook {
            file: crate::notebooks::format::read(text).unwrap().into(),
        })
        .unwrap();
        n.apply(NotebookCommand::ToggleCellType {
            cell_id: "r".into(),
        })
        .unwrap();
        assert_eq!(n.cells()[0].cell_type, CellType::Code);
    }

    #[test]
    fn a_loaded_notebook_numbers_its_runs_from_one() {
        // The file's cells show Out[1] and Out[7] from an earlier session
        let text = r#"{"nbformat": 4, "nbformat_minor": 5, "metadata": {}, "cells": [
            {"id": "a", "cell_type": "code", "source": "x: 1;", "metadata": {}, "execution_count": 1,
             "outputs": [{"output_type": "execute_result", "execution_count": 1, "data": {"text/latex": "1"}, "metadata": {}}]},
            {"id": "b", "cell_type": "code", "source": "y: 2;", "metadata": {}, "execution_count": 7,
             "outputs": [{"output_type": "execute_result", "execution_count": 7, "data": {"text/latex": "2"}, "metadata": {}}]},
            {"id": "c", "cell_type": "code", "source": "%;", "metadata": {}, "execution_count": null, "outputs": []}]}"#;
        let mut n = nb();
        n.apply(NotebookCommand::LoadNotebook {
            file: crate::notebooks::format::read(text).unwrap().into(),
        })
        .unwrap();
        assert!(n.label_map().is_empty());

        // The first run is In[1], whatever the file showed
        run(&mut n, "b", Some("%o5"));
        assert_eq!(
            n.get_cell("b")
                .unwrap()
                .output
                .as_ref()
                .unwrap()
                .execution_count,
            Some(1)
        );
        // %o1 now means that run, not a's stale Out[1]
        assert_eq!(n.label_map().get(&1).map(String::as_str), Some("%o5"));
        assert_eq!(n.previous_output_label("c").as_deref(), Some("%o5"));
        // and a's stale Out[1] is never the output above b
        assert_eq!(n.previous_output_label("b"), None);
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_client_cell_sees_the_nearest_output_above() {
        let mut n = nb();
        let a = n.next_execution_count();
        n.record_client_run("a", a, Some("%o10".into()));
        let b = n.next_execution_count();
        // b failed: it hides nothing above and records no label
        n.record_client_run("b", b, None);
        let ctx = n.client_cell_label_context(&ids(&["a", "b"]));
        assert_eq!(ctx.previous_output_label.as_deref(), Some("%o10"));
        assert_eq!(ctx.label_map.get(&1).map(String::as_str), Some("%o10"));
        assert!(ctx.bare_percent);
    }

    #[test]
    fn a_client_cell_never_sees_a_cell_that_has_not_run() {
        // "old" shows an output from the file, but no run here made it
        let mut n = nb();
        let ctx = n.client_cell_label_context(&ids(&["old"]));
        assert_eq!(ctx.previous_output_label, None);
        let a = n.next_execution_count();
        n.record_client_run("a", a, Some("%o4".into()));
        assert_eq!(
            n.client_cell_label_context(&ids(&["a", "old"]))
                .previous_output_label
                .as_deref(),
            Some("%o4")
        );
    }

    #[test]
    fn a_failed_rerun_hides_the_cells_earlier_output() {
        let mut n = nb();
        let first = n.next_execution_count();
        n.record_client_run("a", first, Some("%o4".into()));
        let second = n.next_execution_count();
        n.record_client_run("a", second, None);
        assert_eq!(
            n.client_cell_label_context(&ids(&["a"]))
                .previous_output_label,
            None
        );
        // %o1 still names the first run, as in the window
        assert_eq!(n.label_map().get(&1).map(String::as_str), Some("%o4"));
    }

    #[test]
    fn a_new_session_forgets_client_runs() {
        let mut n = nb();
        let a = n.next_execution_count();
        n.record_client_run("a", a, Some("%o4".into()));
        n.start_new_session();
        assert_eq!(
            n.client_cell_label_context(&ids(&["a"]))
                .previous_output_label,
            None
        );
        assert_eq!(n.next_execution_count(), 1);
    }
}
