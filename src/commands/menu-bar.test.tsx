// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const invoke = vi.fn(async (..._args: unknown[]) => undefined);
let menuListener: ((e: { payload: { id: string } }) => void) | null = null;
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, fn: (e: { payload: { id: string } }) => void) => {
    if (name === "menu-command") menuListener = fn;
    return () => {};
  }),
}));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("../hooks/useMaxima", () => ({
  useMaxima: () => ({ executeCell: vi.fn(), runAll: vi.fn(), restartSession: vi.fn() }),
}));
const saveActive = vi.fn(async () => {});
vi.mock("../lib/file-commands", () => ({ saveActive: () => saveActive(), saveActiveAs: vi.fn(), openFromDisk: vi.fn() }));

import { useUiStore } from "../store/uiStore";
import { useNotebookStore } from "../store/notebookStore";
import "../lib/i18n";
import { useMenuBar } from "./useMenuBar";

function Host() {
  useMenuBar();
  return null;
}

describe("the menu bar", () => {
  let div: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    invoke.mockClear();
    vi.spyOn(document, "hasFocus").mockReturnValue(true);
    useUiStore.setState({ docsOpen: false });
    // A notebook tab, as View > Docs needs one
    const nb = useNotebookStore.getState();
    nb.addTab("nb-1");
    nb.setActiveTab("nb-1");
    div = document.createElement("div");
    root = createRoot(div);
    await act(async () => root.render(<Host />));
  });

  afterEach(() => act(() => root.unmount()));

  const calls = (name: string) => invoke.mock.calls.filter((c) => c[0] === name);

  it("sends the menu once on load", () => {
    expect(calls("set_menu")).toHaveLength(1);
  });

  it("sends the state, not the menu, when a panel opens", async () => {
    const before = calls("set_menu_state").length;
    await act(async () => useUiStore.getState().setDocsOpen(true));
    expect(calls("set_menu")).toHaveLength(1);
    const sent = calls("set_menu_state").slice(before);
    const last = sent[sent.length - 1];
    expect(last).toBeDefined();
    const states = (last![1] as { states: { id: string; checked: boolean | null }[] }).states;
    expect(states.find((s) => s.id === "view.docs")?.checked).toBe(true);
  });

  it("runs the command a menu item names", async () => {
    await act(async () => menuListener!({ payload: { id: "view.docs" } }));
    expect(useUiStore.getState().docsOpen).toBe(true);
    await act(async () => menuListener!({ payload: { id: "file.save" } }));
    expect(saveActive).toHaveBeenCalledTimes(1);
  });
});
