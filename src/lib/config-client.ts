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
  print_margin_top: number;
  print_margin_bottom: number;
  print_margin_left: number;
  print_margin_right: number;
  backend: string;
  docker_image: string;
  wsl_distro: string;
  container_engine: string;
  mcp_enabled: boolean;
  mcp_listen_address: string;
  mcp_token: string;
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

export function applyPrintMargins(top: number, bottom: number, left: number, right: number): void {
  const id = "print-margins-style";
  let el = document.getElementById(id) as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement("style");
    el.id = id;
    document.head.appendChild(el);
  }
  el.textContent = `@page { margin: ${top}mm ${right}mm ${bottom}mm ${left}mm; }`;
}

export async function setConfig(updates: Partial<AppConfig>): Promise<void> {
  return invoke<void>("set_config", { updates });
}

export async function listWslDistros(): Promise<string[]> {
  return invoke<string[]>("list_wsl_distros");
}

export async function checkWslMaxima(distro: string): Promise<string | null> {
  return invoke<string | null>("check_wsl_maxima", { distro });
}

export interface ClaudeMcpStatus {
  installed: boolean;
  configured: boolean;
}

export async function claudeMcpStatus(): Promise<ClaudeMcpStatus> {
  return invoke<ClaudeMcpStatus>("claude_mcp_status");
}

export async function claudeMcpConfigure(url: string, token: string): Promise<string> {
  return invoke<string>("claude_mcp_configure", { url, token });
}

export interface CodexMcpStatus {
  installed: boolean;
  configured: boolean;
}

export async function codexMcpStatus(): Promise<CodexMcpStatus> {
  return invoke<CodexMcpStatus>("codex_mcp_status");
}

export async function codexMcpConfigure(url: string, token: string): Promise<string> {
  return invoke<string>("codex_mcp_configure", { url, token });
}

export interface GeminiMcpStatus {
  installed: boolean;
  configured: boolean;
}

export async function geminiMcpStatus(): Promise<GeminiMcpStatus> {
  return invoke<GeminiMcpStatus>("gemini_mcp_status");
}

export async function geminiMcpConfigure(url: string, token: string): Promise<string> {
  return invoke<string>("gemini_mcp_configure", { url, token });
}
