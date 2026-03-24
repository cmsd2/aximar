use tauri::State;

use aximar_core::catalog::types::*;
use crate::state::AppState;

#[tauri::command]
pub fn search_functions(state: State<AppState>, query: String) -> Vec<SearchResult> {
    state.catalog.search(&query)
}

#[tauri::command]
pub fn complete_function(state: State<AppState>, prefix: String) -> Vec<CompletionResult> {
    let mut results = state.catalog.complete(&prefix);

    // Also include package functions (lower priority, deduped against catalog)
    let pkg_results = state.packages.complete_functions(&prefix);
    let existing: std::collections::HashSet<String> =
        results.iter().map(|r| r.name.to_lowercase()).collect();
    for r in pkg_results {
        if !existing.contains(&r.name.to_lowercase()) {
            results.push(r);
        }
    }

    results
}

#[tauri::command]
pub fn get_function(state: State<AppState>, name: String) -> Option<MaximaFunction> {
    state.catalog.get(&name).cloned()
}

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> Vec<CategoryGroup> {
    state.catalog.by_category()
}

#[tauri::command]
pub fn get_function_docs(state: State<AppState>, name: String) -> Option<String> {
    state.docs.get(&name).map(|s| s.to_string())
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
