use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub label: String,
    pub template: String,
    pub description: String,
    /// When set, this suggestion triggers a frontend action instead of Maxima evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Where to insert the new cell relative to the current cell: "before" or "after" (default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
}
