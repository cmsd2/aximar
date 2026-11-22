import { beforeEach, describe, expect, it, vi } from "vitest";

const dialog = vi.hoisted(() => ({ answer: "Cancel", calls: [] as unknown[][] }));
const saved = vi.hoisted(() => ({ path: "/work/calc.macnb" as string | null, count: 0 }));
const paths = vi.hoisted(() => [] as [string, string | null][]);

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: vi.fn(async (...args: unknown[]) => {
    dialog.calls.push(args);
    return dialog.answer;
  }),
}));
vi.mock("./notebooks-client", () => ({
  saveNotebook: vi.fn(async () => {
    saved.count += 1;
    return saved.path;
  }),
}));
vi.mock("./notebook-commands", () => ({
  nbSetPath: vi.fn(async (id: string, path: string | null) => {
    paths.push([id, path]);
  }),
  nbUpdateCellInput: vi.fn(),
}));

import { choiceFor, confirmClose } from "./unsaved-changes";
import { useNotebookStore } from "../store/notebookStore";

function setDirty(id: string, dirty: boolean) {
  const store = useNotebookStore.getState();
  if (!store.notebooks[id]) store.addTab(id, `${id}.macnb`);
  const tab = useNotebookStore.getState().notebooks[id];
  useNotebookStore.setState({
    notebooks: { ...useNotebookStore.getState().notebooks, [id]: { ...tab, isDirty: dirty } },
  });
}

const isDirty = (id: string) => useNotebookStore.getState().notebooks[id].isDirty;

beforeEach(() => {
  dialog.answer = "Cancel";
  dialog.calls.length = 0;
  saved.path = "/work/calc.macnb";
  saved.count = 0;
  paths.length = 0;
  setDirty("nb-a", false);
  setDirty("nb-b", false);
});

describe("unsaved changes dialog", () => {
  it("offers Save first and Cancel last, so Return saves and Escape keeps the notebook", async () => {
    setDirty("nb-a", true);
    await confirmClose(["nb-a"]);
    const options = dialog.calls[0][1] as { buttons: Record<string, string> };
    expect(Object.entries(options.buttons)).toEqual([
      ["yes", "Save"],
      ["no", "Don't Save"],
      ["cancel", "Cancel"],
    ]);
    expect(dialog.calls[0][0]).toContain('"nb-a.macnb"');
  });

  it("closes a notebook with no changes without asking", async () => {
    expect(await confirmClose(["nb-a"])).toBe(true);
    expect(dialog.calls).toHaveLength(0);
  });

  it("keeps the notebook open on Cancel or Escape", async () => {
    setDirty("nb-a", true);
    expect(await confirmClose(["nb-a"])).toBe(false);
    expect(saved.count).toBe(0);
    expect(isDirty("nb-a")).toBe(true);
  });

  it("closes without saving on Don't Save", async () => {
    setDirty("nb-a", true);
    dialog.answer = "Don't Save";
    expect(await confirmClose(["nb-a"])).toBe(true);
    expect(saved.count).toBe(0);
  });

  it("saves, records the path and closes on Save", async () => {
    setDirty("nb-a", true);
    dialog.answer = "Save";
    expect(await confirmClose(["nb-a"])).toBe(true);
    expect(saved.count).toBe(1);
    expect(paths).toEqual([["nb-a", "/work/calc.macnb"]]);
    expect(isDirty("nb-a")).toBe(false);
  });

  it("keeps the notebook open when the file dialog is cancelled", async () => {
    setDirty("nb-a", true);
    dialog.answer = "Save";
    saved.path = null;
    expect(await confirmClose(["nb-a"])).toBe(false);
    expect(isDirty("nb-a")).toBe(true);
  });

  it("asks once for several notebooks and saves each with changes", async () => {
    setDirty("nb-a", true);
    setDirty("nb-b", true);
    dialog.answer = "Save";
    expect(await confirmClose(["nb-a", "nb-b"])).toBe(true);
    expect(dialog.calls).toHaveLength(1);
    expect(dialog.calls[0][0]).toContain("2 notebooks");
    expect(saved.count).toBe(2);
  });

  it("treats an unexpected result as Cancel", () => {
    expect(choiceFor("Ok")).toBe("cancel");
    expect(choiceFor("Yes")).toBe("save");
    expect(choiceFor("No")).toBe("discard");
  });
});
