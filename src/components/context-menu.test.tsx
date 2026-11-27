// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const nb = vi.hoisted(() => ({ nbDeleteCell: vi.fn(async () => {}) }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("../hooks/useMaxima", () => ({
  useMaxima: () => ({ executeCell: vi.fn(), runAll: vi.fn(), restartSession: vi.fn() }),
}));
vi.mock("../lib/notebook-commands", async (orig) => ({ ...(await orig<object>()), ...nb }));
// The cells themselves: a stand-in with an output and an editor's text
vi.mock("./Cell", () => ({
  Cell: ({ cell }: { cell: { id: string } }) => (
    <div className="cell" data-id={cell.id}>
      <div className="cm-content">text</div>
      <div className="cell-output">output</div>
    </div>
  ),
}));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { mapSyncCells } from "../hooks/useNotebookEvents";
import { Notebook } from "./Notebook";
import { ContextMenu } from "./ui/ContextMenu";

const right = (el: Element) =>
  act(() => el.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 10, clientY: 10 })));

describe("the context menu", () => {
  let div: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    vi.clearAllMocks();
    div = document.createElement("div");
    document.body.appendChild(div);
    root = createRoot(div);
  });

  afterEach(() => {
    act(() => root.unmount());
    div.remove();
  });

  it("moves between items with the arrows and closes on Escape", () => {
    const onClose = vi.fn();
    const items = ["a", "b", "c"].map((id) => ({ id, label: id, disabled: id === "b", onChoose: vi.fn() }));
    act(() => root.render(<ContextMenu x={0} y={0} items={items} onClose={onClose} />));
    const buttons = div.querySelectorAll("button");
    expect(document.activeElement).toBe(buttons[0]);
    act(() => buttons[0].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })));
    // b is disabled, so the arrow skips it
    expect(document.activeElement).toBe(buttons[2]);
    act(() => buttons[2].dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
    expect(onClose).toHaveBeenCalled();
  });

  describe("on cells", () => {
    beforeEach(() => {
      useNotebookStore.setState({ notebooks: {}, activeNotebookId: null });
      const store = useNotebookStore.getState();
      store.addTab("nb-1");
      store.setActiveTab("nb-1");
      store.applyBackendState(
        "nb-1",
        mapSyncCells(["c1", "c2"].map((id) => ({ id, cell_type: "code", input: id, output: null, status: "idle" }))),
        "notebook_replaced",
      );
      store.setActiveCellId("c1");
      act(() => root.render(<I18nProvider i18n={i18n}><Notebook /></I18nProvider>));
    });

    it("deletes the clicked cell, not the active one", async () => {
      right(div.querySelector('[data-id="c2"] .cell-output')!);
      const del = [...div.querySelectorAll<HTMLButtonElement>(".context-menu button")].find((b) => b.textContent?.startsWith("Delete Cell"));
      expect(del).toBeDefined();
      await act(async () => del!.click());
      expect(nb.nbDeleteCell).toHaveBeenCalledWith("c2");
    });

    it("opens no menu on a right-click in the editor's text", () => {
      right(div.querySelector('[data-id="c2"] .cm-content')!);
      expect(div.querySelector(".context-menu")).toBeNull();
    });
  });
});
