// Tauri's commands, mocked for stories: the window runs in a plain browser
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { completions, docs, functions, packages } from "./catalog-fixture";
import { useNotebookStore } from "../src/store/notebookStore";

export const config = {
  theme: "light", maxima_path: null, font_size: 14, print_font_size: 11, eval_timeout: 30,
  variables_open: true, cell_style: "bracket", autocomplete_mode: "active-hint", show_suggestions: true,
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

/**
 * Answers a story sets for itself, by command, ahead of the ones below.
 * A function is called with the command's arguments, so a story can fail
 * a command by returning a rejected promise.  A story's loader replaces
 * them all, so one story's don't reach the next.
 */
export const storyAnswers: Record<string, unknown> = {};

export function setStoryAnswers(answers: Record<string, unknown>): void {
  for (const key of Object.keys(storyAnswers)) delete storyAnswers[key];
  Object.assign(storyAnswers, answers);
}

export function installTauriMock(): void {
  mockWindows("main");
  mockIPC(
    (cmd, payload) => {
      if (cmd in storyAnswers) {
        const answer = storyAnswers[cmd];
        return typeof answer === "function" ? answer(payload) : answer;
      }
      const args = (payload ?? {}) as Record<string, string>;
      switch (cmd) {
        case "get_config":
          return { config, warnings: [] };
        case "list_templates":
          return templates;
        case "list_variables":
          // The names of the user's variables, as Maxima's values list gives them
          return ["F", "a", "f"];
        case "nb_run_cell":
          // A run that succeeds; the cell's output stays as it was
          return { is_error: false, error: null, duration_ms: 5 };
        case "nb_add_cell":
          return addCell(payload as unknown as AddCell);
        case "get_function":
          return functions[args.name] ?? null;
        case "get_function_docs":
          return docs[args.name] ?? null;
        case "complete_function":
          return completions(args.prefix);
        case "get_package":
          return packages[args.name] ?? null;
        case "package_for_function":
          return Object.values(packages).find((p) => p.functions.includes(args.name))?.name ?? null;
        case "search_functions":
          return Object.values(functions)
            .filter((f) => f.name.includes(args.query) || f.description.includes(args.query))
            .map((f) => ({ function: f, score: 1 }));
        case "get_suggestions":
          // What the core offers after a result
          return [
            { label: "Simplify", template: "ratsimp(%)", description: "Simplify the result" },
            { label: "Integrate", template: "integrate(%, x)", description: "Integrate with respect to x" },
            { label: "Plot", template: "plot2d(%, [x, -5, 5])", description: "Plot the result" },
          ];
        case "list_wsl_distros":
        case "get_buffered_logs":
        case "search_package_functions":
        case "search_packages":
        case "list_categories":
          return [];
        case "mcp_clients_status":
          return {
            claude: { installed: "/usr/local/bin/claude", configured: { state: "unchecked" } },
            codex: { installed: "/usr/local/bin/codex", configured: { state: "yes" } },
            gemini: { installed: null, configured: { state: "no" } },
          };
        case "mcp_server_status":
          return { state: "listening", address: "127.0.0.1:19542" };
        case "maxima_version":
          return "5.50.0";
        case "docs_languages":
          // English, and Spanish with summaries only
          return {
            choice: null,
            effective: "en",
            languages: [
              { tag: "en", full: true, pack: null },
              { tag: "es", full: false, pack: "maxima-core-docs-es" },
            ],
          };
        case "maxima_status":
          return { path: "/opt/homebrew/bin/maxima", source: "standard-location", problem: null };
        case "plugin:event|listen":
          return 1;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}

interface AddCell {
  cellType: string | null;
  input: string | null;
  afterCellId: string | null;
  beforeCellId: string | null;
}

let added = 0;

/** Add a cell to the story's notebook, as the backend's cell_added event would */
function addCell({ cellType, input, afterCellId, beforeCellId }: AddCell): { cell_id: string } {
  const store = useNotebookStore.getState();
  const id = store.activeNotebookId;
  const tab = id ? store.notebooks[id] : undefined;
  if (!id || tab?.type !== "notebook") return { cell_id: "" };
  const cellId = `new-${++added}`;
  const cell = { id: cellId, cellType: (cellType ?? "code") as "code", input: input ?? "", output: null, status: "idle" as const };
  const cells = [...tab.cells];
  const at = afterCellId
    ? cells.findIndex((c) => c.id === afterCellId) + 1
    : beforeCellId
      ? cells.findIndex((c) => c.id === beforeCellId)
      : cells.length;
  cells.splice(at < 0 ? cells.length : at, 0, cell);
  store.applyBackendState(id, cells, "cell_added", cellId);
  return { cell_id: cellId };
}
