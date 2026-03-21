import { invoke } from "@tauri-apps/api/core";
import type { EvalResult } from "../types/maxima";
import type { Suggestion } from "../types/suggestions";

export async function getSuggestions(
  input: string,
  output: EvalResult
): Promise<Suggestion[]> {
  return invoke<Suggestion[]>("get_suggestions", { input, output });
}
