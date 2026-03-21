import { invoke } from "@tauri-apps/api/core";

export interface AppConfig {
  theme: string;
  maxima_path: string | null;
  font_size: number;
  eval_timeout: number;
  variables_open: boolean;
}

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function setConfig(updates: Partial<AppConfig>): Promise<void> {
  return invoke<void>("set_config", { updates });
}
