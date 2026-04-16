use aximar_core::catalog::doc_index::DocIndexStore;
use aximar_core::catalog::docs::Docs;
use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use dashmap::DashMap;
use maxima_mac_parser::MacItem;
use tower_lsp::lsp_types::*;

use crate::document::DocumentState;

pub fn hover_info(
    word: &str,
    catalog: &Catalog,
    docs: &Docs,
    doc_index: &DocIndexStore,
    packages: &PackageCatalog,
    documents: &DashMap<Url, DocumentState>,
) -> Option<Hover> {
    let content = hover_markdown(word, catalog, docs, doc_index, packages, documents)?;
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
    docs: &Docs,
    doc_index: &DocIndexStore,
    packages: &PackageCatalog,
    documents: &DashMap<Url, DocumentState>,
) -> Option<String> {
    // 1. Full docs (most detailed)
    if let Some(doc) = docs.get(word) {
        return Some(doc.to_string());
    }

    // 2. Catalog entry (signatures + description + examples)
    if let Some(func) = catalog.get(word) {
        let mut md = String::new();
        for sig in &func.signatures {
            md.push_str(&format!("```maxima\n{}\n```\n\n", sig));
        }
        md.push_str(&func.description);
        if !func.examples.is_empty() {
            md.push_str("\n\n**Examples:**\n");
            for ex in &func.examples {
                md.push_str(&format!("```maxima\n{}\n```\n", ex.input));
                if let Some(desc) = &ex.description {
                    md.push_str(&format!("{}\n", desc));
                }
            }
        }
        if !func.see_also.is_empty() {
            md.push_str(&format!(
                "\n**See also:** {}",
                func.see_also.join(", ")
            ));
        }
        return Some(md);
    }

    // 3. Doc index (installed packages)
    if let Some(md) = doc_index.hover_markdown(word) {
        return Some(md);
    }

    // 4. Document symbols (user-defined)
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

    // 5. Package info
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
