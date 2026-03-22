use tauri::State;

use crate::catalog::types::*;
use crate::state::AppState;

#[tauri::command]
pub fn search_functions(state: State<AppState>, query: String) -> Vec<SearchResult> {
    state.catalog.search(&query)
}

#[tauri::command]
pub fn complete_function(state: State<AppState>, prefix: String) -> Vec<CompletionResult> {
    state.catalog.complete(&prefix)
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
