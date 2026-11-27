// Notebook files dropped on the window: which of the dropped paths Aximar
// opens, and opening them, each in a tab of its own
import { useNotebookStore } from "../store/notebookStore";
import { nbSetActive } from "./notebook-commands";

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
    const open = tabFor(path);
    try {
      if (open) {
        useNotebookStore.getState().setActiveTab(open);
        await nbSetActive(open);
      } else {
        await openTab(path);
      }
    } catch (e) {
      failed.push({ path, reason: String(e) });
    }
  }
  return failed;
}
