use aximar_core::maxima::types::EvalResult;
use aximar_core::suggestions::rules;
use aximar_core::suggestions::types::Suggestion;

#[tauri::command]
pub fn get_suggestions(input: String, output: EvalResult) -> Vec<Suggestion> {
    rules::suggestions_for_output(&input, &output)
}
