use maxima_mac_parser::MacFile;
use tower_lsp::lsp_types::*;

use crate::convert::parse_error_to_diagnostic;

pub struct DocumentState {
    pub content: String,
    pub version: i32,
    pub parsed: MacFile,
}

impl DocumentState {
    pub fn new(content: String, version: i32) -> Self {
        let parsed = maxima_mac_parser::parse(&content);
        Self {
            content,
            version,
            parsed,
        }
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.parsed
            .errors
            .iter()
            .map(parse_error_to_diagnostic)
            .collect()
    }
}
