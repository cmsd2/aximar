use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Jupyter-compatible notebook format (nbformat 4).
/// Uses `metadata.aximar` namespace for Aximar-specific fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notebook {
    pub nbformat: u32,
    pub nbformat_minor: u32,
    pub metadata: NotebookMetadata,
    pub cells: Vec<NotebookCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookMetadata {
    pub kernelspec: KernelSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aximar: Option<AximarMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelSpec {
    pub name: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AximarMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCell {
    pub cell_type: CellType,
    pub source: CellSource,
    pub metadata: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CellType {
    Code,
    Markdown,
    Raw,
}

/// Cell source can be a single string or an array of strings (per nbformat spec).
/// We normalize to a single string internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellSource {
    String(String),
    Lines(Vec<String>),
}

impl CellSource {
    pub fn as_string(&self) -> String {
        match self {
            CellSource::String(s) => s.clone(),
            CellSource::Lines(lines) => lines.join(""),
        }
    }
}

impl Default for NotebookMetadata {
    fn default() -> Self {
        NotebookMetadata {
            kernelspec: KernelSpec {
                name: "maxima".into(),
                display_name: "Maxima".into(),
                language: Some("maxima".into()),
            },
            aximar: None,
        }
    }
}

impl Notebook {
    pub fn new_empty() -> Self {
        Notebook {
            nbformat: 4,
            nbformat_minor: 0,
            metadata: NotebookMetadata::default(),
            cells: Vec::new(),
        }
    }
}

/// Summary used for template listing (not persisted to disk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub cell_count: usize,
}
