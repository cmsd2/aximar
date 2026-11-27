// Notebook files dropped on the window: which of the dropped paths Aximar
// opens, and opening them, each in a tab of its own
import { useNotebookStore } from "../store/notebookStore";
import { nbClose, nbSetActive } from "./notebook-commands";

const NOTEBOOK = /\.(macnb|ipynb)$/i;

/** The dropped paths Aximar opens (notebooks) and the rest */
export function sortDropped(paths: string[]): { notebooks: string[]; others: string[] } {
  const notebooks = paths.filter((p) => NOTEBOOK.test(p));
  return { notebooks, others: paths.filter((p) => !NOTEBOOK.test(p)) };
}

/** The open tab showing the file at `path`, if any */
function tabFor(path: string): string | undefined {
  const { notebooks } = useNotebookStore.getState();
  return Object.values(notebooks).find((tab) => tab.type === "notebook" && tab.filePath === path)?.id;
}

/**
 * Open each dropped notebook in a tab, or switch to its tab when it is
 * already open.  Returns the files that failed, with the reason, so the
 * caller can say so once for all of them.
 */
export async function openDropped(
  paths: string[],
  openTab: (path: string) => Promise<void>,
): Promise<{ path: string; reason: string }[]> {
  const failed: { path: string; reason: string }[] = [];
  for (const path of sortDropped(paths).notebooks) {
    try {
      await openOrSwitch(path, openTab);
    } catch (e) {
      failed.push({ path, reason: String(e) });
    }
  }
  return failed;
}

/** Switch to the tab showing `path` if one is open, otherwise open it with `openTab` */
export async function openOrSwitch(path: string, openTab: (path: string) => Promise<void>): Promise<void> {
  const open = tabFor(path);
  if (open) {
    useNotebookStore.getState().setActiveTab(open);
    await nbSetActive(open);
  } else {
    await openTab(path);
  }
}

/** Whether a tab is a blank notebook: no file, and no cell with text or output */
function isBlank(id: string): boolean {
  const tab = useNotebookStore.getState().notebooks[id];
  return !!tab && tab.type === "notebook" && !tab.filePath && tab.cells.every((c) => !c.input.trim() && !c.output);
}

/**
 * Open files with `open`, and then close the window's only tab if it was a
 * blank notebook and is still blank, as a document app replaces its empty
 * document.  A file macOS hands Aximar as it starts can arrive after the
 * window has made that tab.
 */
export async function replacingBlankTab<T>(open: () => Promise<T>): Promise<T> {
  const before = Object.keys(useNotebookStore.getState().notebooks);
  const blank = before.length === 1 && isBlank(before[0]) ? before[0] : undefined;
  const result = await open();
  const after = Object.keys(useNotebookStore.getState().notebooks);
  if (blank && after.length > 1 && isBlank(blank)) {
    await nbClose(blank).catch(() => {});
    useNotebookStore.getState().removeTab(blank);
  }
  return result;
}
