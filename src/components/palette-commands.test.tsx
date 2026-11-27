// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const restartSession = vi.fn(async () => {});
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("../hooks/useMaxima", () => ({
  useMaxima: () => ({ executeCell: vi.fn(), runAll: vi.fn(), restartSession: () => restartSession() }),
}));
vi.mock("../lib/catalog-client", () => ({
  searchFunctions: vi.fn(async () => []),
  listCategories: vi.fn(async () => []),
  searchPackages: vi.fn(async () => []),
  searchPackageFunctions: vi.fn(async () => []),
}));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { CommandPalette } from "./CommandPalette";

describe("commands in the palette", () => {
  let div: HTMLDivElement;
  let root: Root;
  const onClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    useNotebookStore.setState({ notebooks: {}, activeNotebookId: null });
    const nb = useNotebookStore.getState();
    nb.addTab("nb-1");
    nb.setActiveTab("nb-1");
    nb.setSessionStatusForNotebook("nb-1", "Ready");
    div = document.createElement("div");
    document.body.appendChild(div);
    root = createRoot(div);
  });

  afterEach(() => {
    act(() => root.unmount());
    div.remove();
  });

  function open(query: string) {
    act(() => root.render(<I18nProvider i18n={i18n}><CommandPalette onClose={onClose} initialQuery={query} /></I18nProvider>));
  }

  const commands = () => [...div.querySelectorAll(".palette-command-label")].map((e) => e.textContent);

  it("finds Restart from \"rest\"", () => {
    open("rest");
    expect(commands()[0]).toBe("Restart");
    expect(div.querySelector(".palette-commands-header")?.textContent).toBe("Commands");
  });

  it("lists only commands after >", () => {
    open(">");
    expect(commands().length).toBeGreaterThan(10);
    expect(div.querySelector(".palette-category-item")).toBeNull();
  });

  it("runs a chosen command and closes", async () => {
    open("restart");
    const input = div.querySelector("input")!;
    await act(async () => input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })));
    expect(onClose).toHaveBeenCalled();
    expect(restartSession).toHaveBeenCalledTimes(1);
  });

  it("leaves out disabled commands", () => {
    // Nothing runs, so Interrupt is disabled
    open(">interrupt");
    expect(commands()).not.toContain("Interrupt");
    act(() => useNotebookStore.getState().setSessionStatusForNotebook("nb-1", "Busy"));
    expect(commands()).toContain("Interrupt");
  });
});
