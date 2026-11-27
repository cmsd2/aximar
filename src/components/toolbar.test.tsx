// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("../hooks/useMaxima", () => ({
  useMaxima: () => ({ executeCell: vi.fn(), runAll: vi.fn(), restartSession: vi.fn() }),
}));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { useUiStore } from "../store/uiStore";
import { setPlatformForTests } from "../commands/shortcut";
import { Toolbar } from "./Toolbar";

describe("the toolbar", () => {
  let div: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    setPlatformForTests("windows");
    useNotebookStore.setState({ notebooks: {}, activeNotebookId: null });
    const nb = useNotebookStore.getState();
    nb.addTab("nb-1");
    nb.setActiveTab("nb-1");
    nb.setSessionStatusForNotebook("nb-1", "Ready");
    useUiStore.setState({ variablesOpen: false, docsOpen: false });
    div = document.createElement("div");
    root = createRoot(div);
    act(() => root.render(<I18nProvider i18n={i18n}><Toolbar /></I18nProvider>));
  });

  afterEach(() => {
    act(() => root.unmount());
    setPlatformForTests(null);
  });

  const button = (name: string) => div.querySelector<HTMLButtonElement>(`button[aria-label="${name}"]`);

  it("shows only the session's commands and the panels", () => {
    const names = [...div.querySelectorAll("button")].map((b) => b.getAttribute("aria-label"));
    expect(names).toEqual(["Run All", "Interrupt", "Restart", "Variables", "Docs", "Log", "Settings..."]);
    expect(div.textContent).not.toMatch(/\+ Cell|\+ Markdown|Undo|Redo|Templates/);
  });

  it("greys out Interrupt while nothing runs, and enables it while a cell runs", () => {
    expect(button("Interrupt")!.disabled).toBe(true);
    act(() => useNotebookStore.getState().setSessionStatusForNotebook("nb-1", "Busy"));
    expect(button("Interrupt")!.disabled).toBe(false);
  });

  it("presses a toggle while its panel is open", () => {
    expect(button("Docs")!.getAttribute("aria-pressed")).toBe("false");
    act(() => useUiStore.getState().setDocsOpen(true));
    expect(button("Docs")!.getAttribute("aria-pressed")).toBe("true");
  });

  it("puts the platform's shortcut in a tooltip", () => {
    expect(button("Interrupt")!.title).toBe("Interrupt (Ctrl+.)");
  });
});
