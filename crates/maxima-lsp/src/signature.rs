use aximar_core::catalog::packages::PackageCatalog;
use aximar_core::catalog::search::Catalog;
use dashmap::DashMap;
use maxima_mac_parser::MacItem;
use tower_lsp::lsp_types::*;

use crate::document::DocumentState;

pub fn signature_help(
    func_name: &str,
    active_param: usize,
    catalog: &Catalog,
    packages: &PackageCatalog,
    documents: &DashMap<Url, DocumentState>,
) -> Option<SignatureHelp> {
    let signatures = build_signatures(func_name, catalog, packages, documents)?;
    Some(SignatureHelp {
        signatures,
        active_signature: Some(0),
        active_parameter: Some(active_param as u32),
    })
}

fn build_signatures(
    func_name: &str,
    catalog: &Catalog,
    packages: &PackageCatalog,
    documents: &DashMap<Url, DocumentState>,
) -> Option<Vec<SignatureInformation>> {
    // 1. Catalog signatures
    if let Some(func) = catalog.get(func_name) {
        let sigs: Vec<SignatureInformation> = func
            .signatures
            .iter()
            .map(|sig| {
                let params = extract_params(sig);
                SignatureInformation {
                    label: sig.clone(),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: func.description.clone(),
                    })),
                    parameters: Some(
                        params
                            .into_iter()
                            .map(|p| ParameterInformation {
                                label: ParameterLabel::Simple(p),
                                documentation: None,
                            })
                            .collect(),
                    ),
                    active_parameter: None,
                }
            })
            .collect();
        if !sigs.is_empty() {
            return Some(sigs);
        }
    }

    // 2. Document symbols (user-defined functions)
    for entry in documents.iter() {
        for item in &entry.value().parsed.items {
            match item {
                MacItem::FunctionDef(f) | MacItem::MacroDef(f) if f.name == func_name => {
                    let label = format!("{}({})", f.name, f.params.join(", "));
                    let doc = f.doc_comment.as_ref().map(|d| {
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: d.clone(),
                        })
                    });
                    return Some(vec![SignatureInformation {
                        label,
                        documentation: doc,
                        parameters: Some(
                            f.params
                                .iter()
                                .map(|p| ParameterInformation {
                                    label: ParameterLabel::Simple(p.clone()),
                                    documentation: None,
                                })
                                .collect(),
                        ),
                        active_parameter: None,
                    }]);
                }
                _ => {}
            }
        }
    }

    // 3. Package functions
    if let Some(pkg_name) = packages.package_for_function(func_name) {
        if let Some(pkg) = packages.get(pkg_name) {
            if let Some(sig) = pkg.signatures.get(func_name) {
                let params = extract_params(sig);
                return Some(vec![SignatureInformation {
                    label: sig.clone(),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("From package `{}`", pkg_name),
                    })),
                    parameters: Some(
                        params
                            .into_iter()
                            .map(|p| ParameterInformation {
                                label: ParameterLabel::Simple(p),
                                documentation: None,
                            })
                            .collect(),
                    ),
                    active_parameter: None,
                }]);
            }
        }
    }

    None
}

/// Extract parameter names from a signature like "func(a, b, c)" or "func(a, b, [opts])".
fn extract_params(sig: &str) -> Vec<String> {
    let open = match sig.find('(') {
        Some(i) => i,
        None => return Vec::new(),
    };
    let close = match sig.rfind(')') {
        Some(i) => i,
        None => return Vec::new(),
    };
    if open + 1 >= close {
        return Vec::new();
    }
    let inner = &sig[open + 1..close];
    inner
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_params_basic() {
        assert_eq!(
            extract_params("integrate(f, x)"),
            vec!["f".to_string(), "x".to_string()]
        );
    }

    #[test]
    fn extract_params_empty() {
        assert_eq!(extract_params("quit()"), Vec::<String>::new());
    }

    #[test]
    fn extract_params_variadic() {
        assert_eq!(
            extract_params("f(a, [opts])"),
            vec!["a".to_string(), "[opts]".to_string()]
        );
    }
}
