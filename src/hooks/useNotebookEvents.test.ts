import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ label: "main" }) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

import { handleLifecycleEvent, handleNotebookStateEvent } from "./useNotebookEvents";
import { useNotebookStore } from "../store/notebookStore";

describe("handleLifecycleEvent", () => {
  beforeEach(() => {
    const store = useNotebookStore.getState();
    for (const id of ["nb-a", "nb-b"]) {
      if (!store.notebooks[id]) store.addTab(id);
    }
    useNotebookStore.getState().setActiveTab("nb-a");
  });

  it("follows an active notebook switch made over MCP", () => {
    handleLifecycleEvent("nb-b", "active_changed");
    expect(useNotebookStore.getState().activeNotebookId).toBe("nb-b");
  });

  it("ignores a switch to a notebook this window doesn't hold", () => {
    handleLifecycleEvent("nb-elsewhere", "active_changed");
    expect(useNotebookStore.getState().activeNotebookId).toBe("nb-a");
  });

  it("adopts a notebook created over MCP", () => {
    handleLifecycleEvent("nb-new", "created");
    expect(useNotebookStore.getState().notebooks["nb-new"]).toBeDefined();
  });
});

describe("handleNotebookStateEvent", () => {
  const blocked = (notebookId: string) => ({
    notebook_id: notebookId,
    effect: "trust_required",
    cell_id: "cell-1",
    cells: [],
    can_undo: false,
    can_redo: false,
    trusted: false,
    trust_functions: ["load"],
  });

  beforeEach(() => {
    const store = useNotebookStore.getState();
    for (const id of ["nb-a", "nb-b"]) {
      if (!store.notebooks[id]) store.addTab(id);
      store.setPendingTrustFunctions(id, null);
    }
    useNotebookStore.getState().setActiveTab("nb-a");
  });

  it("shows the trust banner for a run blocked over MCP", () => {
    handleNotebookStateEvent(blocked("nb-a"));
    const tab = useNotebookStore.getState().notebooks["nb-a"];
    expect(tab.type === "notebook" && tab.pendingTrustFunctions).toEqual(["load"]);
  });

  it("puts the banner on the blocked notebook, not the active one", () => {
    handleNotebookStateEvent(blocked("nb-b"));
    const { notebooks } = useNotebookStore.getState();
    const a = notebooks["nb-a"];
    const b = notebooks["nb-b"];
    expect(a.type === "notebook" && a.pendingTrustFunctions).toBeNull();
    expect(b.type === "notebook" && b.pendingTrustFunctions).toEqual(["load"]);
  });

  it("leaves the cells and the saved state alone", () => {
    const before = useNotebookStore.getState().notebooks["nb-a"];
    handleNotebookStateEvent(blocked("nb-a"));
    const after = useNotebookStore.getState().notebooks["nb-a"];
    expect(after.type === "notebook" && after.cells).toEqual(before.type === "notebook" && before.cells);
    expect(after.type === "notebook" && after.isDirty).toBe(before.type === "notebook" && before.isDirty);
  });
});

describe("notebook paths", () => {
  const event = (effect: string, extra: Record<string, unknown> = {}) => ({
    notebook_id: "nb-a",
    effect,
    cell_id: null,
    cells: [],
    can_undo: false,
    can_redo: false,
    trusted: true,
    ...extra,
  });
  const tab = () => {
    const t = useNotebookStore.getState().notebooks["nb-a"];
    if (t.type !== "notebook") throw new Error("not a notebook tab");
    return t;
  };

  beforeEach(() => {
    const store = useNotebookStore.getState();
    if (!store.notebooks["nb-a"]) store.addTab("nb-a");
    store.setNotebookPath("nb-a", null);
  });

  it("takes the title and save path from path_changed", () => {
    handleNotebookStateEvent(event("path_changed", { path: "/work/calc.macnb" }));
    expect(tab().filePath).toBe("/work/calc.macnb");
    expect(tab().title).toBe("calc.macnb");
  });

  it("clears them when the path is cleared", () => {
    handleNotebookStateEvent(event("path_changed", { path: "/work/calc.macnb" }));
    handleNotebookStateEvent(event("path_changed"));
    expect(tab().filePath).toBeNull();
    // An unsaved notebook has no title of its own; the tab shows "Untitled"
    expect(tab().title).toBe("");
  });

  it("keeps the path when the cells are replaced", () => {
    handleNotebookStateEvent(event("path_changed", { path: "C:\\work\\demo.macnb" }));
    handleNotebookStateEvent(event("notebook_replaced"));
    expect(tab().filePath).toBe("C:\\work\\demo.macnb");
    expect(tab().title).toBe("demo.macnb");
  });
});
