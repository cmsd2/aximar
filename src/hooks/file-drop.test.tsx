// @vitest-environment happy-dom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type Handler = (event: { payload: { type: string; paths?: string[] } }) => Promise<void> | void;
let dragHandler: Handler | undefined;
const openNotebookTab = vi.fn(async (path: string) => {
  if (path.includes("broken")) throw new Error("not a notebook file");
});
const message = vi.fn(async () => {});
const nbSetActive = vi.fn(async (_id: string) => {});

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: async (handler: Handler) => {
      dragHandler = handler;
      return () => {};
    },
  }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: (...args: unknown[]) => message(...(args as [])) }));
vi.mock("./useNotebookEvents", () => ({ openNotebookTab: (path: string) => openNotebookTab(path) }));
vi.mock("../lib/notebook-commands", () => ({ nbSetActive: (id: string) => nbSetActive(id) }));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { openDropped, sortDropped } from "../lib/file-drop";
import { useFileDrop, type DragState } from "./useFileDrop";

// The error dialog's title is translated text
i18n.activate("en");

describe("dropped files", () => {
  it("opens .macnb and .ipynb files, in any case, and nothing else", () => {
    expect(sortDropped(["/a/one.macnb", "/a/two.IPYNB", "/a/plot.json", "/a/notes.txt"])).toEqual({
      notebooks: ["/a/one.macnb", "/a/two.IPYNB"],
      others: ["/a/plot.json", "/a/notes.txt"],
    });
  });

  it("switches to a notebook that is already open instead of opening it again", async () => {
    const store = useNotebookStore.getState();
    store.addTab("nb-open");
    store.setNotebookPath("nb-open", "/a/open.macnb");
    store.addTab("nb-other");
    store.setActiveTab("nb-other");
    openNotebookTab.mockClear();
    await openDropped(["/a/open.macnb"], openNotebookTab);
    expect(openNotebookTab).not.toHaveBeenCalled();
    expect(useNotebookStore.getState().activeNotebookId).toBe("nb-open");
    expect(nbSetActive).toHaveBeenCalledWith("nb-open");
  });

  it("opens the rest when one fails, and reports the one", async () => {
    openNotebookTab.mockClear();
    const failed = await openDropped(["/a/broken.macnb", "/a/good.macnb"], openNotebookTab);
    expect(openNotebookTab).toHaveBeenCalledTimes(2);
    expect(failed).toEqual([{ path: "/a/broken.macnb", reason: "Error: not a notebook file" }]);
  });
});

describe("dragging onto the window", () => {
  let drag: DragState | null = null;

  function Probe() {
    drag = useFileDrop();
    return null;
  }

  beforeEach(async () => {
    openNotebookTab.mockClear();
    message.mockClear();
    drag = null;
    const div = document.createElement("div");
    await act(async () => createRoot(div).render(<Probe />));
  });

  it("counts what is dragged in, and forgets it on leave", async () => {
    await act(async () => dragHandler!({ payload: { type: "enter", paths: ["/a/x.macnb", "/a/y.txt"] } }));
    expect(drag).toEqual({ notebooks: 1, others: 1 });
    await act(async () => dragHandler!({ payload: { type: "leave" } }));
    expect(drag).toBeNull();
  });

  it("opens each notebook dropped, and only notebooks", async () => {
    await act(async () => dragHandler!({ payload: { type: "drop", paths: ["/a/x.macnb", "/a/y.ipynb", "/a/z.txt"] } }));
    expect(openNotebookTab.mock.calls.map((c) => c[0])).toEqual(["/a/x.macnb", "/a/y.ipynb"]);
    expect(drag).toBeNull();
    expect(message).not.toHaveBeenCalled();
  });

  it("says which files failed, once", async () => {
    await act(async () => dragHandler!({ payload: { type: "drop", paths: ["/a/broken.macnb"] } }));
    expect(message).toHaveBeenCalledTimes(1);
  });
});
