use tauri::State;

use aximar_core::maxima::types::EvalResult;
use aximar_core::suggestions::rules;
use aximar_core::suggestions::types::Suggestion;

use crate::state::AppState;

#[tauri::command]
pub fn get_suggestions(
    state: State<AppState>,
    input: String,
    output: EvalResult,
) -> Vec<Suggestion> {
    rules::suggestions_for_output_with_packages(&input, &output, Some(&state.packages))
}
