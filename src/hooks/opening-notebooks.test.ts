import { beforeEach, describe, expect, it, vi } from "vitest";

const backend = vi.hoisted(() => ({ openError: null as string | null, calls: [] as unknown[][] }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ label: "main" }) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("../lib/notebook-commands", () => ({
  nbCreate: vi.fn(async () => ({ notebook_id: "nb-new" })),
  nbSetActive: vi.fn(async () => {}),
  nbOpen: vi.fn(async (path: string, id: string) => {
    backend.calls.push(["nb_open", path, id]);
    if (backend.openError) throw backend.openError;
  }),
  nbClose: vi.fn(async (id: string) => {
    backend.calls.push(["nb_close", id]);
  }),
  nbUpdateCellInput: vi.fn(),
}));

import { handleNotebookStateEvent, mapSyncCells, openNotebookTab } from "./useNotebookEvents";
import type { NotebookStateEvent } from "./useNotebookEvents";
import type { SyncCell, SyncCellOutput } from "../lib/notebook-commands";
import { useNotebookStore } from "../store/notebookStore";

const divisionByZero = {
  title: "Division by Zero",
  explanation: "You divided by zero.",
  suggestion: null,
  example: null,
  did_you_mean: [],
  correct_signatures: [],
};

function syncCell(output: Partial<SyncCellOutput>): SyncCell {
  return {
    id: "c1",
    cell_type: "code",
    input: "1/0;",
    status: "idle",
    output: {
      text_output: "",
      latex: null,
      plot_svg: null,
      plot_data: null,
      image_png: null,
      error: null,
      is_error: false,
      duration_ms: 0,
      output_label: null,
      execution_count: 1,
      text_plain: null,
      error_info: null,
      stale: false,
      ...output,
    },
  };
}

function event(effect: string, cells: SyncCell[]): NotebookStateEvent {
  return { notebook_id: "nb-a", effect, cell_id: null, cells, can_undo: false, can_redo: false, trusted: false };
}

function tab(id: string) {
  const t = useNotebookStore.getState().notebooks[id];
  if (!t || t.type !== "notebook") throw new Error(`no notebook tab ${id}`);
  return t;
}

beforeEach(() => {
  backend.openError = null;
  backend.calls.length = 0;
  const store = useNotebookStore.getState();
  for (const id of Object.keys(store.notebooks)) store.removeTab(id);
  store.addTab("nb-a", "calc.macnb");
});

describe("outputs from the backend", () => {
  it("carry a loaded error's explanation and staleness to the cell", () => {
    const [cell] = mapSyncCells([
      syncCell({ error: "expt: undefined: 0 to a negative exponent.", is_error: true, error_info: divisionByZero, stale: true }),
    ]);
    expect(cell.output?.errorInfo?.title).toBe("Division by Zero");
    expect(cell.output?.stale).toBe(true);
  });

  it("carry the result's one-line form", () => {
    const [cell] = mapSyncCells([syncCell({ text_plain: "x^2/2", latex: "{{x^2}\\over{2}}" })]);
    expect(cell.output?.textPlain).toBe("x^2/2");
    expect(cell.output?.stale).toBe(false);
  });

  it("keep raw cells raw", () => {
    const [cell] = mapSyncCells([{ ...syncCell({}), cell_type: "raw", output: null }]);
    expect(cell.cellType).toBe("raw");
  });
});

describe("opening a file", () => {
  it("leaves the notebook unchanged, unless cells needed new IDs", () => {
    handleNotebookStateEvent(event("notebook_replaced", [syncCell({ stale: true })]));
    expect(tab("nb-a").isDirty).toBe(false);
    expect(tab("nb-a").cells[0].output?.stale).toBe(true);

    handleNotebookStateEvent(event("ids_replaced", [syncCell({ stale: true })]));
    expect(tab("nb-a").isDirty).toBe(true);
  });

  it("opens a notebook file in a new tab named after it", async () => {
    await openNotebookTab("/work/orbits.macnb");
    expect(backend.calls).toEqual([["nb_open", "/work/orbits.macnb", "nb-new"]]);
    expect(useNotebookStore.getState().activeNotebookId).toBe("nb-new");
    expect(tab("nb-new").title).toBe("orbits.macnb");
  });

  it("removes the new tab when the file can't be opened", async () => {
    backend.openError = "Failed to open /work/old.ipynb: nbformat 3 isn't supported";
    await expect(openNotebookTab("/work/old.ipynb")).rejects.toBe(backend.openError);
    expect(backend.calls).toEqual([["nb_open", "/work/old.ipynb", "nb-new"], ["nb_close", "nb-new"]]);
    expect(useNotebookStore.getState().notebooks["nb-new"]).toBeUndefined();
  });
});
