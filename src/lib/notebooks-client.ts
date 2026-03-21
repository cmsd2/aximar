import { invoke } from "@tauri-apps/api/core";
import type { Notebook, TemplateSummary } from "../types/notebooks";

export async function listTemplates(): Promise<TemplateSummary[]> {
  return invoke<TemplateSummary[]>("list_templates");
}

export async function getTemplate(id: string): Promise<Notebook | null> {
  return invoke<Notebook | null>("get_template", { id });
}

export async function getHasSeenWelcome(): Promise<boolean> {
  return invoke<boolean>("get_has_seen_welcome");
}

export async function setHasSeenWelcome(): Promise<void> {
  return invoke<void>("set_has_seen_welcome");
}
