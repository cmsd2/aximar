use dashmap::DashMap;
use maxima_mac_parser::MacItem;
use tower_lsp::lsp_types::*;

use crate::convert::parser_span_to_lsp;
use crate::document::DocumentState;

pub fn goto_definition(
    word: &str,
    documents: &DashMap<Url, DocumentState>,
    current_uri: &Url,
) -> Option<Location> {
    // Search current document first
    if let Some(loc) = find_definition_in_doc(word, documents, current_uri) {
        return Some(loc);
    }

    // Search other open documents
    for entry in documents.iter() {
        if entry.key() == current_uri {
            continue;
        }
        if let Some(loc) = find_definition_in_doc(word, documents, entry.key()) {
            return Some(loc);
        }
    }

    None
}

fn find_definition_in_doc(
    word: &str,
    documents: &DashMap<Url, DocumentState>,
    uri: &Url,
) -> Option<Location> {
    let doc = documents.get(uri)?;
    for item in &doc.parsed.items {
        let (name, name_span) = match item {
            MacItem::FunctionDef(f) | MacItem::MacroDef(f) => (&f.name, f.name_span),
            MacItem::VariableAssign(v) => (&v.name, v.name_span),
        };
        if name == word {
            return Some(Location {
                uri: uri.clone(),
                range: parser_span_to_lsp(name_span),
            });
        }
    }
    None
}

pub fn find_references(
    word: &str,
    documents: &DashMap<Url, DocumentState>,
) -> Vec<Location> {
    let mut locations = Vec::new();

    for entry in documents.iter() {
        let uri = entry.key();
        let content = &entry.value().content;

        // Simple whole-word text search
        let content_bytes = content.as_bytes();

        let mut search_from = 0;
        while let Some(pos) = content[search_from..].find(word) {
            let abs_pos = search_from + pos;
            let before_ok = abs_pos == 0
                || !is_ident_char(content_bytes[abs_pos - 1] as char);
            let after_pos = abs_pos + word.len();
            let after_ok = after_pos >= content_bytes.len()
                || !is_ident_char(content_bytes[after_pos] as char);

            if before_ok && after_ok {
                // Convert byte offset to line/character
                if let Some(range) = byte_offset_to_range(content, abs_pos, word.len()) {
                    locations.push(Location {
                        uri: uri.clone(),
                        range,
                    });
                }
            }
            search_from = abs_pos + word.len().max(1);
        }
    }

    locations
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '%' || c == '?'
}

fn byte_offset_to_range(content: &str, offset: usize, len: usize) -> Option<Range> {
    let start = byte_offset_to_position(content, offset)?;
    let end = byte_offset_to_position(content, offset + len)?;
    Some(Range { start, end })
}

fn byte_offset_to_position(content: &str, offset: usize) -> Option<Position> {
    let mut line = 0u32;
    let mut line_start = 0usize;

    for (i, ch) in content.char_indices() {
        if i == offset {
            let character = utf16_len(&content[line_start..offset]);
            return Some(Position { line, character });
        }
        if ch == '\n' {
            line += 1;
            line_start = i + 1;
        }
    }

    if offset == content.len() {
        let character = utf16_len(&content[line_start..offset]);
        return Some(Position { line, character });
    }

    None
}

fn utf16_len(s: &str) -> u32 {
    s.chars()
        .map(|ch| if (ch as u32) > 0xFFFF { 2u32 } else { 1u32 })
        .sum()
}
