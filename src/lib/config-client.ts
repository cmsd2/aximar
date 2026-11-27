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
  /** The language in use, such as `es`: the user's choice, or the system's */
  language: string;
  /** The languages with a translation, English first */
  languages: string[];
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

/** Whether an AI client's configuration lists Aximar's MCP server */
export type ConfigState =
  | { state: "yes" }
  | { state: "no" }
  | { state: "unchecked" }
  | { state: "timed-out" }
  | { state: "error"; file: string; detail: string };

export interface ClientStatus {
  /** Where the client's command is, or null if it isn't installed */
  installed: string | null;
  configured: ConfigState;
}

export interface McpClientsStatus {
  claude: ClientStatus;
  codex: ClientStatus;
  gemini: ClientStatus;
}

export type McpClient = keyof McpClientsStatus;

/** Every client's status.  Runs none of their programs. */
export async function mcpClientsStatus(): Promise<McpClientsStatus> {
  return invoke<McpClientsStatus>("mcp_clients_status");
}

/** Ask Claude Code whether it lists Aximar: runs `claude mcp get aximar` */
export async function claudeMcpCheck(): Promise<ConfigState> {
  return invoke<ConfigState>("claude_mcp_check");
}

const CONFIGURE_COMMANDS: Record<McpClient, string> = {
  claude: "claude_mcp_configure",
  codex: "codex_mcp_configure",
  gemini: "gemini_mcp_configure",
};

/** Add Aximar's MCP server to a client's configuration */
export async function mcpConfigure(client: McpClient, url: string, token: string): Promise<string> {
  return invoke<string>(CONFIGURE_COMMANDS[client], { url, token });
}
