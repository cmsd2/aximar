use std::collections::HashSet;

use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use dashmap::DashMap;
use maxima_mac_parser::MacItem;
use tower_lsp::lsp_types::*;

use crate::document::DocumentState;

pub fn completions(
    prefix: &str,
    catalog: &Catalog,
    packages: &PackageCatalog,
    documents: &DashMap<Url, DocumentState>,
    _current_uri: &Url,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();

    // 1. Catalog completions (built-in functions)
    for cr in catalog.complete(prefix) {
        if seen.insert(cr.name.clone()) {
            items.push(CompletionItem {
                label: cr.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(cr.signature.clone()),
                documentation: if cr.description.is_empty() {
                    None
                } else {
                    Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: cr.description.clone(),
                    }))
                },
                insert_text: Some(cr.insert_text.clone()),
                ..Default::default()
            });
        }
    }

    // 2. Package function completions
    for cr in packages.complete_functions(prefix) {
        if seen.insert(cr.name.clone()) {
            let detail = if let Some(pkg) = &cr.package {
                format!("{} ({})", cr.signature, pkg)
            } else {
                cr.signature.clone()
            };
            items.push(CompletionItem {
                label: cr.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail),
                documentation: if cr.description.is_empty() {
                    None
                } else {
                    Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: cr.description.clone(),
                    }))
                },
                insert_text: Some(cr.insert_text.clone()),
                ..Default::default()
            });
        }
    }

    // 3. Document symbol completions (user-defined functions/variables)
    for entry in documents.iter() {
        for item in &entry.value().parsed.items {
            let (name, kind) = match item {
                MacItem::FunctionDef(f) | MacItem::MacroDef(f) => {
                    (&f.name, CompletionItemKind::FUNCTION)
                }
                MacItem::VariableAssign(v) => (&v.name, CompletionItemKind::VARIABLE),
            };
            if name.starts_with(prefix) && seen.insert(name.clone()) {
                let detail = match item {
                    MacItem::FunctionDef(f) | MacItem::MacroDef(f) => {
                        Some(format!("{}({})", f.name, f.params.join(", ")))
                    }
                    _ => None,
                };
                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(kind),
                    detail,
                    ..Default::default()
                });
            }
        }
    }

    items.truncate(50);
    items
}
