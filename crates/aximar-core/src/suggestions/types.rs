use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub label: String,
    pub template: String,
    pub description: String,
    /// When set, this suggestion triggers a frontend action instead of Maxima evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
}
