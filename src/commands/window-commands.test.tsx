// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const close = vi.fn(async () => {});
const nb = vi.hoisted(() => ({
  nbUndo: vi.fn(async () => {}),
  nbRedo: vi.fn(async () => {}),
  nbDeleteCell: vi.fn(async () => {}),
  nbClose: vi.fn(async () => {}),
  nbSetActive: vi.fn(async () => {}),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("../lib/notebook-commands", async (orig) => ({ ...(await orig<object>()), ...nb }));
vi.mock("../lib/unsaved-changes", () => ({ confirmClose: vi.fn(async () => true), saveNotebookTab: vi.fn() }));
const openNewNotebook = vi.fn();
vi.mock("../lib/new-notebook", () => ({ openNewNotebook: () => openNewNotebook() }));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { TabBar } from "../components/TabBar";
import { undoOrRedo } from "./registry";
import { closeActiveTab } from "../lib/tab-commands";
import { setPlatformForTests } from "./shortcut";

function tabs(ids: string[]) {
  useNotebookStore.setState({ notebooks: {}, activeNotebookId: null });
  const s = useNotebookStore.getState();
  for (const id of ids) s.addTab(id);
  s.setActiveTab(ids[0]);
}

const press = (key: string, mods: KeyboardEventInit = {}) =>
  act(() => window.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, ...mods })));

describe("keys the window still catches", () => {
  let div: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    setPlatformForTests("macos");
    vi.clearAllMocks();
    tabs(["a", "b"]);
    div = document.createElement("div");
    document.body.appendChild(div);
    root = createRoot(div);
    act(() => root.render(<I18nProvider i18n={i18n}><TabBar /></I18nProvider>));
  });

  afterEach(() => {
    act(() => root.unmount());
    div.remove();
    setPlatformForTests(null);
  });

  it("switch tabs on Ctrl+Tab and back on Ctrl+Shift+Tab", async () => {
    await press("Tab", { ctrlKey: true });
    expect(useNotebookStore.getState().activeNotebookId).toBe("b");
    await press("Tab", { ctrlKey: true, shiftKey: true });
    expect(useNotebookStore.getState().activeNotebookId).toBe("a");
  });

  it("leave Mod+T and Mod+D alone", async () => {
    await press("t", { metaKey: true });
    await press("d", { metaKey: true });
    expect(openNewNotebook).not.toHaveBeenCalled();
    expect(nb.nbDeleteCell).not.toHaveBeenCalled();
  });
});

describe("undo", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  it("undoes the notebook from a cell", async () => {
    document.body.innerHTML = '<div class="cell"><div class="cm-content" contenteditable="true" tabindex="0"></div></div>';
    (document.querySelector(".cm-content") as HTMLElement).focus();
    undoOrRedo("undo");
    // After sending any typing still waiting (see undo.test.ts)
    await vi.waitFor(() => expect(nb.nbUndo).toHaveBeenCalledTimes(1));
  });

  it("undoes the find bar's own text", () => {
    document.body.innerHTML = '<div class="find-bar"><input class="find-bar-input" /></div>';
    (document.querySelector("input") as HTMLInputElement).focus();
    const exec = vi.fn(() => true);
    (document as unknown as { execCommand: typeof exec }).execCommand = exec;
    nb.nbUndo.mockClear();
    undoOrRedo("undo");
    expect(exec).toHaveBeenCalledWith("undo");
    expect(nb.nbUndo).not.toHaveBeenCalled();
  });
});

describe("Close Tab", () => {
  beforeEach(() => vi.clearAllMocks());

  it("closes one of two tabs and leaves the window", async () => {
    tabs(["a", "b"]);
    await closeActiveTab();
    expect(Object.keys(useNotebookStore.getState().notebooks)).toEqual(["b"]);
    expect(close).not.toHaveBeenCalled();
  });

  it("closes the window on the last tab", async () => {
    tabs(["a"]);
    await closeActiveTab();
    expect(close).toHaveBeenCalledTimes(1);
    expect(Object.keys(useNotebookStore.getState().notebooks)).toEqual(["a"]);
  });
});
