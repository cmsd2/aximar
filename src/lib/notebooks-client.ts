import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import { t } from "@lingui/core/macro";
import type { TemplateSummary } from "../types/notebooks";
import { flushNotebook } from "./dirty-inputs";
import { nbSave } from "./notebook-commands";

// Notebook files are read and written by the backend (aximar-core's
// notebooks::format), the same way for the window, MCP and batch runs.
// This module only picks paths and asks the backend.

// "Plotly JSON" names a file format, so it stays as it is in every language
const PLOTLY_FILTER = { name: "Plotly JSON", extensions: ["json"] };

/** The file types a notebook saves as, named in the window's language */
function notebookFilters() {
  return [
    {
      name: t({
        message: "Maxima Notebook",
        comment: "File type in the Open and Save dialogs, for .macnb files",
      }),
      extensions: ["macnb"],
    },
    {
      name: t({
        message: "Jupyter Notebook",
        comment: "File type in the Open and Save dialogs, for .ipynb files",
      }),
      extensions: ["ipynb"],
    },
  ];
}

function allOpenFilters() {
  return [
    {
      name: t({
        message: "All Supported",
        comment: "File type in the Open dialog that shows every kind of file Aximar opens",
      }),
      extensions: ["macnb", "ipynb", "json"],
    },
    ...notebookFilters(),
    PLOTLY_FILTER,
  ];
}

export async function listTemplates(): Promise<TemplateSummary[]> {
  return invoke<TemplateSummary[]>("list_templates");
}

export async function getHasSeenWelcome(): Promise<boolean> {
  return invoke<boolean>("get_has_seen_welcome");
}

export async function setHasSeenWelcome(): Promise<void> {
  return invoke<void>("set_has_seen_welcome");
}

/** Save a notebook to `path`, after sending the backend any input edits
 *  still waiting on the debounce timer.  The `path_changed` event then sets
 *  the tab's title and save path. */
async function saveTo(notebookId: string, path: string): Promise<void> {
  await flushNotebook(notebookId);
  await nbSave(path, notebookId);
}

/**
 * Save the notebook to its file, or ask for one with a Save dialog.
 * Returns the path saved to, or null if cancelled.
 */
export async function saveNotebook(
  notebookId: string,
  filePath: string | null,
): Promise<string | null> {
  const path =
    filePath ??
    (await save({
      defaultPath: "notebook.macnb",
      filters: notebookFilters(),
    }));
  if (!path) return null;
  await saveTo(notebookId, path);
  return path;
}

/**
 * Save As: always shows a dialog regardless of the current file.
 * Returns the path saved to, or null if cancelled.
 */
export async function saveNotebookAs(
  notebookId: string,
  currentPath: string | null,
): Promise<string | null> {
  const path = await save({
    defaultPath: currentPath ?? "notebook.macnb",
    filters: notebookFilters(),
  });
  if (!path) return null;
  await saveTo(notebookId, path);
  return path;
}

/** A file to open: a notebook for the backend to load, or a plot read here. */
export type OpenFileResult =
  | { type: "notebook"; path: string }
  | { type: "plot"; plotData: string; path: string };

/**
 * Show an Open dialog and pick a file.
 * Returns a notebook path or plot, or null if cancelled.
 */
export async function openFile(): Promise<OpenFileResult | null> {
  const selected = await open({
    multiple: false,
    filters: allOpenFilters(),
  });
  if (!selected) return null;
  return openFilePath(selected);
}

/**
 * Open a file by path (no dialog). Used for CLI file arguments.
 */
export async function openFilePath(path: string): Promise<OpenFileResult> {
  if (path.endsWith(".json")) {
    const plotData = await invoke<string>("read_text_file", { path });
    return { type: "plot", plotData, path };
  }
  return { type: "notebook", path };
}

/** Save a Plotly JSON string to a file (always shows dialog). Returns the saved path, or null if cancelled. */
export async function savePlotFile(
  plotData: string,
  defaultPath: string | null,
): Promise<string | null> {
  const path = await save({
    defaultPath: defaultPath ?? "plot.json",
    filters: [PLOTLY_FILTER],
  });
  if (!path) return null;

  await invoke("write_text_file", { path, content: plotData });
  return path;
}
