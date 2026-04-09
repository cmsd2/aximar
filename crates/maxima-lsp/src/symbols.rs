use maxima_mac_parser::MacItem;
use tower_lsp::lsp_types::*;

use crate::convert::parser_span_to_lsp;

#[allow(deprecated)] // DocumentSymbol::deprecated field
pub fn mac_item_to_document_symbol(item: &MacItem) -> DocumentSymbol {
    match item {
        MacItem::FunctionDef(f) => DocumentSymbol {
            name: f.name.clone(),
            detail: Some(format!("({})", f.params.join(", "))),
            kind: SymbolKind::FUNCTION,
            range: parser_span_to_lsp(f.span),
            selection_range: parser_span_to_lsp(f.name_span),
            children: None,
            tags: None,
            deprecated: None,
        },
        MacItem::MacroDef(f) => DocumentSymbol {
            name: f.name.clone(),
            detail: Some(format!("macro({})", f.params.join(", "))),
            kind: SymbolKind::FUNCTION,
            range: parser_span_to_lsp(f.span),
            selection_range: parser_span_to_lsp(f.name_span),
            children: None,
            tags: None,
            deprecated: None,
        },
        MacItem::VariableAssign(v) => DocumentSymbol {
            name: v.name.clone(),
            detail: None,
            kind: SymbolKind::VARIABLE,
            range: parser_span_to_lsp(v.span),
            selection_range: parser_span_to_lsp(v.name_span),
            children: None,
            tags: None,
            deprecated: None,
        },
    }
}

pub fn document_symbols(items: &[MacItem]) -> Vec<DocumentSymbol> {
    items.iter().map(mac_item_to_document_symbol).collect()
}
