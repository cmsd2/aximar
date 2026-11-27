// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => null) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("../hooks/useMaxima", () => ({
  useMaxima: () => ({ executeCell: vi.fn(), runAll: vi.fn(), restartSession: vi.fn() }),
}));
vi.mock("react-virtuoso", () => ({ Virtuoso: () => null }));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { useFindStore } from "../store/findStore";
import { useLogStore } from "../store/logStore";
import { mapSyncCells } from "../hooks/useNotebookEvents";
import { Toolbar } from "./Toolbar";
import { TabBar } from "./TabBar";
import { Notebook } from "./Notebook";
import { FindBar } from "./FindBar";
import { VariablePanel } from "./VariablePanel";
import { DocsPanel } from "./DocsPanel";
import { LogWindow } from "./LogPanel";

/** The characters that used to stand in for icons */
const GLYPHS = /^[▲▼▶■×←→↔✎✓+]$|^Aa$/;

describe("icon buttons", () => {
  let div: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    useNotebookStore.setState({ notebooks: {}, activeNotebookId: null });
    const nb = useNotebookStore.getState();
    nb.addTab("nb-1");
    nb.addTab("nb-2");
    nb.setActiveTab("nb-1");
    nb.applyBackendState(
      "nb-1",
      mapSyncCells([
        { id: "c1", cell_type: "code", input: "x+1;", output: null, status: "running" },
        { id: "m1", cell_type: "markdown", input: "# Notes", output: null, status: "idle" },
        { id: "r1", cell_type: "raw", input: "raw", output: null, status: "idle" },
      ]),
      "notebook_replaced",
    );
    useFindStore.getState().open(true);
    useLogStore.getState().openWindow();
    div = document.createElement("div");
    document.body.appendChild(div);
    root = createRoot(div);
    act(() =>
      root.render(
        <I18nProvider i18n={i18n}>
          <TabBar />
          <Toolbar />
          <FindBar />
          <VariablePanel open />
          <Notebook />
          <DocsPanel open requestId={0} onClose={() => {}} />
          <LogWindow />
        </I18nProvider>,
      ),
    );
  });

  afterEach(() => {
    act(() => root.unmount());
    div.remove();
    useFindStore.getState().close();
    useLogStore.getState().closeWindow();
  });

  it("show no character as an icon, and each has a name", () => {
    const buttons = [...div.querySelectorAll("button")];
    expect(buttons.length).toBeGreaterThan(15);
    for (const b of buttons) {
      const text = (b.textContent ?? "").trim();
      expect(text, b.outerHTML.slice(0, 120)).not.toMatch(GLYPHS);
      const name = b.getAttribute("aria-label") || b.getAttribute("title") || text;
      expect(name, b.outerHTML.slice(0, 120)).toBeTruthy();
    }
  });
});
