// Seeds the notebook store for stories: one tab, with cells and trust
import { useNotebookStore } from "../store/notebookStore";
import { mapSyncCells } from "../hooks/useNotebookEvents";
import type { SyncCell } from "../lib/notebook-commands";

export function seedNotebook(
  cells: SyncCell[] = [],
  options: { trusted?: boolean; pendingTrust?: string[]; dirty?: boolean } = {},
): void {
  const store = useNotebookStore.getState();
  if (!store.notebooks["nb-1"]) store.addTab("nb-1");
  store.setActiveTab("nb-1");
  store.applyBackendState("nb-1", mapSyncCells(cells), "notebook_replaced", undefined, true, false, options.trusted ?? true);
  store.setPendingTrustFunctions("nb-1", options.pendingTrust ?? null);
  // A story's caret or selection would otherwise stay for the next story
  store.setInsertionPoint(null);
  store.setSelectedCellIds([]);
  if (options.dirty) store.markNotebookDirty("nb-1");
}
