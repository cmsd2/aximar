use std::collections::HashSet;

use tauri::State;

use aximar_core::catalog::doc_index::SymbolEntry;
use aximar_core::catalog::types::*;
use crate::state::AppState;

/// Convert a doc-index SymbolEntry to a MaximaFunction for frontend compatibility.
fn symbol_to_function(name: &str, entry: &SymbolEntry) -> MaximaFunction {
    let mut sigs = vec![entry.signature.clone()];
    for alt in &entry.signatures {
        if !sigs.contains(alt) {
            sigs.push(alt.clone());
        }
    }
    MaximaFunction {
        name: name.to_string(),
        signatures: sigs,
        description: if entry.body_md.is_empty() {
            entry.summary.clone()
        } else {
            entry.body_md.clone()
        },
        category: entry
            .category
            .as_deref()
            .and_then(parse_category)
            .unwrap_or(FunctionCategory::Other),
        examples: entry
            .examples
            .iter()
            .map(|e| FunctionExample {
                input: e.input.clone(),
                description: if e.description.is_empty() {
                    None
                } else {
                    Some(e.description.clone())
                },
            })
            .collect(),
        see_also: entry.see_also.clone(),
        search_keywords: entry.keywords.join(" "),
    }
}

#[tauri::command]
pub fn search_functions(state: State<AppState>, query: String) -> Vec<SearchResult> {
    state
        .catalog
        .search(&query)
        .into_iter()
        .filter_map(|dr| {
            state.catalog.get(&dr.name).map(|(_, entry)| SearchResult {
                function: symbol_to_function(&dr.name, entry),
                score: dr.score,
            })
        })
        .collect()
}

#[tauri::command]
pub fn complete_function(state: State<AppState>, prefix: String) -> Vec<CompletionResult> {
    let mut results = state.catalog.complete(&prefix);
    let existing: HashSet<String> = results.iter().map(|r| r.name.to_lowercase()).collect();

    // Also include package functions (lower priority, deduped)
    for r in state.packages.complete_functions(&prefix) {
        if !existing.contains(&r.name.to_lowercase()) {
            results.push(r);
        }
    }

    results
}

#[tauri::command]
pub fn get_function(state: State<AppState>, name: String) -> Option<MaximaFunction> {
    state
        .catalog
        .get(&name)
        .map(|(_, entry)| symbol_to_function(&name, entry))
}

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> Vec<CategoryGroup> {
    let doc_cats = state.catalog.by_category();
    doc_cats
        .into_iter()
        .map(|g| {
            let category = parse_category(&g.category).unwrap_or(FunctionCategory::Other);
            CategoryGroup {
                label: g.category,
                category,
                functions: g
                    .symbols
                    .into_iter()
                    .filter_map(|(name, _sig)| {
                        state.catalog.get(&name).map(|(_, entry)| {
                            MaximaFunction {
                                name,
                                signatures: vec![entry.signature.clone()],
                                description: entry.summary.clone(),
                                category,
                                examples: Vec::new(),
                                see_also: Vec::new(),
                                search_keywords: String::new(),
                            }
                        })
                    })
                    .collect(),
            }
        })
        .collect()
}

#[tauri::command]
pub fn get_function_docs(state: State<AppState>, name: String) -> Option<String> {
    state
        .catalog
        .get(&name)
        .and_then(|(_, entry)| {
            if entry.body_md.is_empty() {
                None
            } else {
                Some(entry.body_md.clone())
            }
        })
}

// ── Package commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn search_packages(state: State<AppState>, query: String) -> Vec<PackageSearchResult> {
    state.packages.search(&query)
}

#[tauri::command]
pub fn complete_packages(state: State<AppState>, prefix: String) -> Vec<PackageCompletionResult> {
    state.packages.complete(&prefix)
}

#[tauri::command]
pub fn get_package(state: State<AppState>, name: String) -> Option<PackageInfo> {
    state.packages.get(&name).cloned()
}

#[tauri::command]
pub fn package_for_function(state: State<AppState>, name: String) -> Option<String> {
    state.packages.package_for_function(&name).map(|s| s.to_string())
}

#[tauri::command]
pub fn search_package_functions(state: State<AppState>, query: String) -> Vec<PackageFunctionSearchResult> {
    state.packages.search_functions(&query)
}

/// Parse a string category into the FunctionCategory enum.
fn parse_category(s: &str) -> Option<FunctionCategory> {
    match s {
        "Calculus" => Some(FunctionCategory::Calculus),
        "Algebra" => Some(FunctionCategory::Algebra),
        "LinearAlgebra" | "Linear Algebra" => Some(FunctionCategory::LinearAlgebra),
        "Simplification" => Some(FunctionCategory::Simplification),
        "Solving" => Some(FunctionCategory::Solving),
        "Plotting" => Some(FunctionCategory::Plotting),
        "Trigonometry" => Some(FunctionCategory::Trigonometry),
        "NumberTheory" | "Number Theory" => Some(FunctionCategory::NumberTheory),
        "Polynomials" => Some(FunctionCategory::Polynomials),
        "Series" => Some(FunctionCategory::Series),
        "Combinatorics" => Some(FunctionCategory::Combinatorics),
        "Programming" => Some(FunctionCategory::Programming),
        "IO" | "I/O" => Some(FunctionCategory::IO),
        "Other" => Some(FunctionCategory::Other),
        _ => None,
    }
}
