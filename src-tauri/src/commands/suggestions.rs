use crate::maxima::types::EvalResult;
use crate::suggestions::rules;
use crate::suggestions::types::Suggestion;

#[tauri::command]
pub fn get_suggestions(input: String, output: EvalResult) -> Vec<Suggestion> {
    rules::suggestions_for_output(&input, &output)
}
