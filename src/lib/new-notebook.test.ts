import { describe, expect, it, vi } from "vitest";

const nbCreate = vi.fn(async (_options?: { empty?: boolean }) => ({ notebook_id: "nb-new" }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("./notebook-commands", () => ({
  nbCreate: (options?: { empty?: boolean }) => nbCreate(options),
  nbSetActive: vi.fn(async () => {}),
  nbGetState: vi.fn(async () => ({
    notebook_id: "nb-new", effect: "notebook_replaced", cell_id: null, cells: [], can_undo: false, can_redo: false, trusted: true,
  })),
}));

import { useNotebookStore } from "../store/notebookStore";
import { openNewNotebook } from "./new-notebook";

describe("a new notebook", () => {
  it("opens empty in a new tab, with the caret ready for the first cell", async () => {
    await openNewNotebook();
    expect(nbCreate).toHaveBeenCalledWith({ empty: true });
    const state = useNotebookStore.getState();
    expect(state.activeNotebookId).toBe("nb-new");
    const tab = state.notebooks["nb-new"];
    expect(tab.type === "notebook" && tab.cells).toEqual([]);
    expect(tab.type === "notebook" && tab.insertionPoint).toEqual({ after: null });
  });
});
