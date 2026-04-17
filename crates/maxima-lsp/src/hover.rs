use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use dashmap::DashMap;
use maxima_mac_parser::MacItem;
use tower_lsp::lsp_types::*;

use crate::document::DocumentState;

pub fn hover_info(
    word: &str,
    catalog: &Catalog,
    packages: &PackageCatalog,
    documents: &DashMap<Url, DocumentState>,
) -> Option<Hover> {
    let content = hover_markdown(word, catalog, packages, documents)?;
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: None,
    })
}

fn hover_markdown(
    word: &str,
    catalog: &Catalog,
    packages: &PackageCatalog,
    documents: &DashMap<Url, DocumentState>,
) -> Option<String> {
    // 1. Catalog hover (preferred — includes core + installed packages via doc-index)
    if let Some(md) = catalog.hover_markdown(word) {
        return Some(md);
    }

    // 2. Document symbols (user-defined)
    for entry in documents.iter() {
        for item in &entry.value().parsed.items {
            match item {
                MacItem::FunctionDef(f) | MacItem::MacroDef(f) if f.name == word => {
                    let mut md = format!(
                        "```maxima\n{}({})\n```\n",
                        f.name,
                        f.params.join(", ")
                    );
                    if let Some(doc) = &f.doc_comment {
                        md.push_str(&format!("\n{}", doc));
                    }
                    return Some(md);
                }
                MacItem::VariableAssign(v) if v.name == word => {
                    return Some(format!("Variable `{}`", v.name));
                }
                _ => {}
            }
        }
    }

    // 3. Package info
    if let Some(pkg_name) = packages.package_for_function(word) {
        if let Some(pkg) = packages.get(pkg_name) {
            let sig = pkg
                .signatures
                .get(word)
                .cloned()
                .unwrap_or_else(|| format!("{}()", word));
            return Some(format!(
                "```maxima\n{}\n```\n\nFrom package `{}`: {}",
                sig, pkg.name, pkg.description
            ));
        }
    }

    None
}
