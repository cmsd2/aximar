import { invoke } from "@tauri-apps/api/core";
import { useNotebookStore } from "../store/notebookStore";

/** Get the currently active notebook ID for passing to backend commands. */
function activeId(): string | null {
  return useNotebookStore.getState().activeNotebookId;
}

export async function listVariables(): Promise<string[]> {
  return invoke<string[]>("list_variables", { notebookId: activeId() });
}

export async function killVariable(name: string): Promise<void> {
  return invoke<void>("kill_variable", { notebookId: activeId(), name });
}

export async function killAllVariables(): Promise<void> {
  return invoke<void>("kill_all_variables", { notebookId: activeId() });
}
