import { invoke } from "@tauri-apps/api/core";

export async function listVariables(): Promise<string[]> {
  return invoke<string[]>("list_variables");
}

export async function killVariable(name: string): Promise<void> {
  return invoke<void>("kill_variable", { name });
}

export async function killAllVariables(): Promise<void> {
  return invoke<void>("kill_all_variables");
}
