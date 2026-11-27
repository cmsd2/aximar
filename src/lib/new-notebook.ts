// New notebook, from the tab bar's + and File > New: an empty notebook in
// a new tab, with the caret ready for the first cell
import { useNotebookStore } from "../store/notebookStore";
import { nbCreate, nbGetState, nbSetActive } from "./notebook-commands";

export async function openNewNotebook(): Promise<string> {
  const store = useNotebookStore.getState();
  const { notebook_id } = await nbCreate({ empty: true });
  store.addTab(notebook_id);
  store.setActiveTab(notebook_id);
  await nbSetActive(notebook_id);
  const state = await nbGetState(notebook_id);
  store.applyBackendState(
    state.notebook_id,
    state.cells.map((sc) => ({
      id: sc.id,
      cellType: sc.cell_type as "code" | "markdown",
      input: sc.input,
      output: null,
      status: "idle" as const,
    })),
    state.effect,
    state.cell_id ?? undefined,
    state.can_undo,
    state.can_redo,
  );
  // Typing starts the first cell (see CellGap)
  useNotebookStore.getState().setInsertionPoint({ after: null });
  return notebook_id;
}
