// @vitest-environment happy-dom
// Files the system asks Aximar to open (a drop on the Dock icon, Open
// With, a double-click): queued by the backend, taken by the main window
import { beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let queued: string[] = [];
let created = 0;
const opened: string[] = [];
const listeners: Record<string, () => void> = {};

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    switch (cmd) {
      case "get_initial_file_args":
        return null;
      case "take_pending_opens": {
        const taken = queued;
        queued = [];
        return taken;
      }
      case "nb_create":
        return { notebook_id: `nb-${++created}` };
      case "nb_open":
        opened.push(args!.path as string);
        return null;
      case "nb_list":
        return [];
      default:
        return null;
    }
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: () => void) => {
    listeners[name] = handler;
    return () => {};
  }),
}));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ label: "main" }) }));

import { useNotebookStore } from "../store/notebookStore";
import { useNotebookEvents } from "./useNotebookEvents";

function Probe() {
  useNotebookEvents();
  return null;
}

async function settle() {
  for (let i = 0; i < 10; i++) await act(async () => {});
}

describe("files the system asks Aximar to open", () => {
  beforeEach(() => {
    queued = [];
    opened.length = 0;
  });

  it("opens a file that launched Aximar, instead of an empty notebook", async () => {
    queued = ["/Users/ana/notes.macnb"];
    await act(async () => createRoot(document.createElement("div")).render(<Probe />));
    await settle();
    expect(opened).toEqual(["/Users/ana/notes.macnb"]);
    const tabs = Object.values(useNotebookStore.getState().notebooks);
    // Only the file's tab: no empty notebook beside it
    expect(tabs.map((t) => t.title)).toEqual(["notes.macnb"]);
  });

  it("opens a file dropped on the Dock icon while Aximar runs", async () => {
    await act(async () => createRoot(document.createElement("div")).render(<Probe />));
    await settle();
    queued = ["/tmp/lab.ipynb"];
    await act(async () => listeners["files-waiting"]());
    await settle();
    expect(opened).toEqual(["/tmp/lab.ipynb"]);
  });

  it("switches to a file that is already open instead of opening it twice", async () => {
    await act(async () => createRoot(document.createElement("div")).render(<Probe />));
    await settle();
    queued = ["/tmp/twice.macnb"];
    await act(async () => listeners["files-waiting"]());
    await settle();
    const id = useNotebookStore.getState().activeNotebookId!;
    // The backend's path_changed event sets the path in the app
    useNotebookStore.getState().setNotebookPath(id, "/tmp/twice.macnb");
    queued = ["/tmp/twice.macnb"];
    await act(async () => listeners["files-waiting"]());
    await settle();
    expect(opened).toEqual(["/tmp/twice.macnb"]);
    expect(useNotebookStore.getState().activeNotebookId).toBe(id);
  });
});
