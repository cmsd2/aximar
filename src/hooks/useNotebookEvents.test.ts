import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ label: "main" }) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

import { handleLifecycleEvent } from "./useNotebookEvents";
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
