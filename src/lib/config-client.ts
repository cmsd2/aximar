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
  /** The Settings section shown last, such as `printing` */
  settings_section: string;
  /** The help's language, or null to follow the app's */
  docs_language?: string | null;
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
  mono: "var(--font-mono)",
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

/** The Maxima the Local backend would run, and where it came from */
export interface MaximaLocation {
  path: string | null;
  source: "setting" | "environment" | "standard-location" | "path" | null;
  problem: "not-found" | "not-executable" | null;
}

/** Which Maxima the Local backend would run with this Maxima path setting.  Starts nothing. */
export async function maximaStatus(maximaPath: string | null): Promise<MaximaLocation> {
  return invoke<MaximaLocation>("maxima_status", { maximaPath });
}

/** The version a Maxima program reports, such as 5.47.0: runs it once with --version */
export async function maximaVersion(path: string): Promise<string | null> {
  return invoke<string | null>("maxima_version", { path });
}

/** Show the open-source licences Aximar ships, in their own window */
export async function openLicences(): Promise<void> {
  return invoke<void>("open_licences");
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

/** What Aximar's MCP server is doing */
export type McpServerState =
  | { state: "off" }
  | { state: "starting"; address: string }
  | { state: "listening"; address: string }
  | { state: "failed"; address: string; reason: string };

export async function mcpServerStatus(): Promise<McpServerState> {
  return invoke<McpServerState>("mcp_server_status");
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
