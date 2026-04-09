use maxima_mac_parser::{MacFile, MacItem};
use tower_lsp::lsp_types::*;

pub fn folding_ranges(content: &str, parsed: &MacFile) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();

    // 1. Function/macro definitions
    for item in &parsed.items {
        let span = match item {
            MacItem::FunctionDef(f) | MacItem::MacroDef(f) => f.span,
            MacItem::VariableAssign(v) => v.span,
        };
        if span.end.line > span.start.line {
            ranges.push(FoldingRange {
                start_line: span.start.line,
                start_character: Some(span.start.character),
                end_line: span.end.line,
                end_character: Some(span.end.character),
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: None,
            });
        }
    }

    // 2. Multi-line block comments
    let mut i = 0;
    let bytes = content.as_bytes();
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            let mut depth = 1;
            i += 2;
            while i + 1 < bytes.len() && depth > 0 {
                if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let end = i;
            let start_line = content[..start].matches('\n').count() as u32;
            let end_line = content[..end].matches('\n').count() as u32;
            if end_line > start_line {
                ranges.push(FoldingRange {
                    start_line,
                    start_character: None,
                    end_line,
                    end_character: None,
                    kind: Some(FoldingRangeKind::Comment),
                    collapsed_text: None,
                });
            }
        } else {
            i += 1;
        }
    }

    ranges
}
