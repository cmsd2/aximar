// Tauri's commands, mocked for stories: the window runs in a plain browser
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

export const config = {
  theme: "light", maxima_path: null, font_size: 14, print_font_size: 11, eval_timeout: 30,
  variables_open: true, cell_style: "bracket", autocomplete_mode: "active-hint",
  markdown_font: "sans-serif", markdown_indent: "aligned", print_margin_top: 15,
  print_margin_bottom: 15, print_margin_left: 15, print_margin_right: 15, backend: "local",
  docker_image: "aximar/maxima", wsl_distro: "Ubuntu", container_engine: "docker",
  mcp_enabled: true, mcp_listen_address: "127.0.0.1:19542", mcp_token: "0123456789abcdef",
  language: "en", languages: ["en", "es"], settings_section: "general",
};

const templates = [
  { id: "calculus", title: "Calculus", description: "Derivatives, integrals and limits", cell_count: 12 },
  { id: "linear-algebra", title: "Linear algebra", description: "Matrices and vectors", cell_count: 1 },
];

export function installTauriMock(): void {
  mockWindows("main");
  mockIPC(
    (cmd) => {
      switch (cmd) {
        case "get_config":
          return { config, warnings: [] };
        case "list_templates":
          return templates;
        case "list_variables":
          return [
            { name: "x", value: "42" },
            { name: "f", value: "f(t):=t^2" },
          ];
        case "list_wsl_distros":
        case "get_suggestions":
        case "search_functions":
        case "search_packages":
        case "list_categories":
          return [];
        case "mcp_clients_status":
          return {
            claude: { installed: "/usr/local/bin/claude", configured: { state: "unchecked" } },
            codex: { installed: "/usr/local/bin/codex", configured: { state: "yes" } },
            gemini: { installed: null, configured: { state: "no" } },
          };
        case "plugin:event|listen":
          return 1;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
