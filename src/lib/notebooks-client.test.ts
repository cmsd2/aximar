import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const calls = vi.hoisted(() => [] as unknown[][]);
const dialog = vi.hoisted(() => ({ savePath: "/work/new.macnb" as string | null }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (command: string, args: Record<string, unknown>) => {
    calls.push([command, args]);
    if (command === "read_text_file") return '{"data": []}';
  }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn(async () => dialog.savePath),
  open: vi.fn(),
}));

import { cleanup, markDirty } from "./dirty-inputs";
import { openFilePath, saveNotebook, saveNotebookAs } from "./notebooks-client";

beforeEach(() => {
  calls.length = 0;
  dialog.savePath = "/work/new.macnb";
});

// Edits left on the debounce timer don't fire into later tests
afterEach(() => cleanup());

describe("saving a notebook", () => {
  it("sends waiting edits to the backend, then saves that notebook", async () => {
    markDirty("nb-a", "cell-1", "x: 1;");
    markDirty("nb-b", "cell-2", "y: 2;");
    expect(await saveNotebook("nb-a", "/work/calc.macnb")).toBe("/work/calc.macnb");
    expect(calls).toEqual([
      ["nb_update_cell_input", { notebookId: "nb-a", cellId: "cell-1", input: "x: 1;" }],
      ["nb_save", { notebookId: "nb-a", path: "/work/calc.macnb" }],
    ]);
  });

  it("asks for a file when the notebook has none", async () => {
    expect(await saveNotebook("nb-a", null)).toBe("/work/new.macnb");
    expect(calls).toContainEqual(["nb_save", { notebookId: "nb-a", path: "/work/new.macnb" }]);
  });

  it("saves nothing when the file dialog is cancelled", async () => {
    dialog.savePath = null;
    expect(await saveNotebook("nb-a", null)).toBeNull();
    expect(await saveNotebookAs("nb-a", "/work/calc.macnb")).toBeNull();
    expect(calls.filter(([c]) => c === "nb_save")).toEqual([]);
  });

  it("Save As always asks, and saves to the new file", async () => {
    expect(await saveNotebookAs("nb-a", "/work/calc.macnb")).toBe("/work/new.macnb");
    expect(calls).toContainEqual(["nb_save", { notebookId: "nb-a", path: "/work/new.macnb" }]);
  });
});

describe("opening a file", () => {
  it("leaves notebooks for the backend to read", async () => {
    expect(await openFilePath("/work/calc.ipynb")).toEqual({ type: "notebook", path: "/work/calc.ipynb" });
    expect(calls).toEqual([]);
  });

  it("reads a Plotly JSON file as a plot", async () => {
    expect(await openFilePath("/work/plot.json")).toEqual({ type: "plot", plotData: '{"data": []}', path: "/work/plot.json" });
  });
});
