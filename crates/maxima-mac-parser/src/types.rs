/// A parsed `.mac` file.
#[derive(Debug, Clone, PartialEq)]
pub struct MacFile {
    pub items: Vec<MacItem>,
    pub load_calls: Vec<LoadCall>,
    pub errors: Vec<ParseError>,
}

/// A top-level item extracted from a `.mac` file.
#[derive(Debug, Clone, PartialEq)]
pub enum MacItem {
    FunctionDef(FunctionDef),
    MacroDef(FunctionDef),
    VariableAssign(VariableAssign),
}

/// A function or macro definition (`f(x) := ...` or `f(x) ::= ...`).
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub span: Span,
    pub name_span: Span,
    /// Line where the body begins (after `:=`), for DAP breakpoint offset calculation.
    pub body_start_line: u32,
    /// A `/* ... */` comment immediately preceding the definition.
    pub doc_comment: Option<String>,
    /// Local variable names from `block([var1, var2, ...], ...)` if the
    /// function body starts with a `block` expression.
    pub block_locals: Vec<String>,
}

/// A top-level variable assignment (`name : ...`).
#[derive(Debug, Clone, PartialEq)]
pub struct VariableAssign {
    pub name: String,
    pub span: Span,
    pub name_span: Span,
}

/// A `load("path")` call.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadCall {
    pub path: String,
    pub span: Span,
}

/// A range in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

/// A position in the source text (0-based line and character).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// 0-based line number.
    pub line: u32,
    /// 0-based character offset (UTF-16 code units, per LSP convention).
    pub character: u32,
}

/// Classification of a parse error.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParseErrorKind {
    #[error("unterminated string literal")]
    UnterminatedString,

    #[error("unterminated block comment")]
    UnterminatedComment,

    #[error("unexpected character '{0}'")]
    UnexpectedChar(char),

    #[error("expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String },

    #[error("skipped malformed statement")]
    SkippedStatement,

    #[error("missing statement terminator ($ or ;)")]
    MissingTerminator,

    #[error("{0}")]
    Other(String),
}

/// Severity of a parse error, for LSP diagnostic mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// An error encountered during parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
    pub severity: Severity,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.kind)
    }
}

impl ParseError {
    /// Returns the human-readable error message.
    pub fn message(&self) -> String {
        self.kind.to_string()
    }
}

impl MacItem {
    pub fn name(&self) -> &str {
        match self {
            MacItem::FunctionDef(f) | MacItem::MacroDef(f) => &f.name,
            MacItem::VariableAssign(v) => &v.name,
        }
    }

    pub fn span(&self) -> Span {
        match self {
            MacItem::FunctionDef(f) | MacItem::MacroDef(f) => f.span,
            MacItem::VariableAssign(v) => v.span,
        }
    }

    pub fn name_span(&self) -> Span {
        match self {
            MacItem::FunctionDef(f) | MacItem::MacroDef(f) => f.name_span,
            MacItem::VariableAssign(v) => v.name_span,
        }
    }
}
