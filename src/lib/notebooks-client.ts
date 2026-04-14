import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import type { Notebook, NotebookCell, NotebookMetadata } from "../types/notebooks";
import type { TemplateSummary } from "../types/notebooks";
import type { Cell } from "../types/notebook";

const NOTEBOOK_FILTERS = [
  { name: "Maxima Notebook", extensions: ["macnb"] },
  { name: "Jupyter Notebook", extensions: ["ipynb"] },
];

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

/** Convert frontend Cell[] to Jupyter nbformat cells for saving. */
function cellsToNotebookCells(cells: Cell[]): NotebookCell[] {
  return cells.map((cell) => ({
    cell_type: cell.cellType === "markdown" ? "markdown" : "code",
    source: cell.input,
    metadata: {},
    ...(cell.cellType === "code"
      ? { execution_count: null, outputs: [] }
      : {}),
  }));
}

function buildNotebook(cells: Cell[]): Notebook {
  const metadata: NotebookMetadata = {
    kernelspec: {
      name: "maxima",
      display_name: "Maxima",
      language: "maxima",
    },
  };
  return {
    nbformat: 4,
    nbformat_minor: 0,
    metadata,
    cells: cellsToNotebookCells(cells),
  };
}

/**
 * Save the notebook to the given path (or prompt with Save As dialog).
 * Returns the path saved to, or null if cancelled.
 */
export async function saveNotebook(
  cells: Cell[],
  filePath: string | null,
): Promise<string | null> {
  const path =
    filePath ??
    (await save({
      defaultPath: "notebook.macnb",
      filters: NOTEBOOK_FILTERS,
    }));
  if (!path) return null;

  const notebook = buildNotebook(cells);
  await invoke("save_notebook", { path, notebook });
  return path;
}

/**
 * Show an Open dialog and load the selected notebook.
 * Returns { notebook, path } or null if cancelled.
 */
export async function openNotebook(): Promise<{
  notebook: Notebook;
  path: string;
} | null> {
  const selected = await open({
    multiple: false,
    filters: NOTEBOOK_FILTERS,
  });
  if (!selected) return null;

  const path = typeof selected === "string" ? selected : selected;
  const notebook = await invoke<Notebook>("open_notebook", { path });
  return { notebook, path };
}

/**
 * Save As: always shows a dialog regardless of current filePath.
 * Returns the path saved to, or null if cancelled.
 */
export async function saveNotebookAs(
  cells: Cell[],
  currentPath: string | null,
): Promise<string | null> {
  const defaultPath = currentPath ?? "notebook.macnb";
  const path = await save({
    defaultPath,
    filters: NOTEBOOK_FILTERS,
  });
  if (!path) return null;

  const notebook = buildNotebook(cells);
  await invoke("save_notebook", { path, notebook });
  return path;
}
