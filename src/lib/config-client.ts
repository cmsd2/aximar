import { invoke } from "@tauri-apps/api/core";

export interface AppConfig {
  theme: string;
  maxima_path: string | null;
  font_size: number;
  print_font_size: number;
  eval_timeout: number;
  variables_open: boolean;
  cell_style: string;
  autocomplete_mode: string;
  markdown_font: string;
  markdown_indent: string;
}

export interface ConfigResponse {
  config: AppConfig;
  warnings: string[];
}

export async function getConfig(): Promise<ConfigResponse> {
  return invoke<ConfigResponse>("get_config");
}

const MARKDOWN_FONT_STACKS: Record<string, string> = {
  "sans-serif":
    '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif',
  serif: 'Georgia, "Times New Roman", Times, serif',
  "computer-modern": '"CMU Serif", Georgia, serif',
  mono: '"JetBrains Mono", "Fira Code", "SF Mono", Menlo, Consolas, monospace',
};

export function markdownFontStack(value: string): string {
  return MARKDOWN_FONT_STACKS[value] ?? MARKDOWN_FONT_STACKS["sans-serif"];
}

export async function setConfig(updates: Partial<AppConfig>): Promise<void> {
  return invoke<void>("set_config", { updates });
}
