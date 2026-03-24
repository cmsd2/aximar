use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use aximar_core::maxima::output::OutputEvent;
use aximar_core::maxima::types::EvalResult;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellType {
    Code,
    Markdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellStatus {
    Idle,
    Running,
    Success,
    Error,
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
pub struct McpCell {
    pub id: String,
    pub cell_type: CellType,
    pub input: String,
    pub output: Option<CellOutput>,
    pub status: CellStatus,
    pub raw_output: Vec<OutputEvent>,
}

pub struct McpNotebook {
    cells: Vec<McpCell>,
    execution_counter: u32,
    /// Maps display count → real Maxima %oN label
    label_map: HashMap<u32, String>,
}

impl McpNotebook {
    pub fn new() -> Self {
        let initial_cell = McpCell {
            id: new_cell_id(),
            cell_type: CellType::Code,
            input: String::new(),
            output: None,
            status: CellStatus::Idle,
            raw_output: Vec::new(),
        };
        McpNotebook {
            cells: vec![initial_cell],
            execution_counter: 0,
            label_map: HashMap::new(),
        }
    }

    pub fn cells(&self) -> &[McpCell] {
        &self.cells
    }

    pub fn get_cell(&self, id: &str) -> Option<&McpCell> {
        self.cells.iter().find(|c| c.id == id)
    }

    pub fn get_cell_mut(&mut self, id: &str) -> Option<&mut McpCell> {
        self.cells.iter_mut().find(|c| c.id == id)
    }

    pub fn add_cell(&mut self, cell_type: CellType, input: String, after_cell_id: Option<&str>) -> String {
        let cell = McpCell {
            id: new_cell_id(),
            cell_type,
            input,
            output: None,
            status: CellStatus::Idle,
            raw_output: Vec::new(),
        };
        let id = cell.id.clone();

        if let Some(after_id) = after_cell_id {
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
        self.cells.push(McpCell {
            id: new_cell_id(),
            cell_type: CellType::Code,
            input: String::new(),
            output: None,
            status: CellStatus::Idle,
            raw_output: Vec::new(),
        });
    }
}

fn new_cell_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("cell-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}
