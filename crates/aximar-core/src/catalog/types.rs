use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaximaFunction {
    pub name: String,
    pub signatures: Vec<String>,
    pub description: String,
    pub category: FunctionCategory,
    pub examples: Vec<FunctionExample>,
    #[serde(default)]
    pub see_also: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExample {
    pub input: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionCategory {
    Calculus,
    Algebra,
    LinearAlgebra,
    Simplification,
    Solving,
    Plotting,
    Trigonometry,
    NumberTheory,
    Polynomials,
    Series,
    Combinatorics,
    Programming,
    IO,
    Other,
}

impl FunctionCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Calculus => "Calculus",
            Self::Algebra => "Algebra",
            Self::LinearAlgebra => "Linear Algebra",
            Self::Simplification => "Simplification",
            Self::Solving => "Solving",
            Self::Plotting => "Plotting",
            Self::Trigonometry => "Trigonometry",
            Self::NumberTheory => "Number Theory",
            Self::Polynomials => "Polynomials",
            Self::Series => "Series",
            Self::Combinatorics => "Combinatorics",
            Self::Programming => "Programming",
            Self::IO => "I/O",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub function: MaximaFunction,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResult {
    pub name: String,
    pub signature: String,
    pub description: String,
    pub insert_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryGroup {
    pub category: FunctionCategory,
    pub label: String,
    pub functions: Vec<MaximaFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationInfo {
    pub name: String,
    pub description: String,
    pub replacement: Option<String>,
}
