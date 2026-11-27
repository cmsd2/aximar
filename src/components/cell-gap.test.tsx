// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

// Tell React this is a test, so act() flushes without warnings
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const nbAddCell = vi.fn(async () => ({ cell_id: "new" }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../lib/notebook-commands", () => ({ nbAddCell: (...args: unknown[]) => nbAddCell(...(args as [])) }));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { fromGap, gapBeside, typeFor } from "../lib/cell-caret";
import type { Cell } from "../types/notebook";
import { CellGap } from "./CellGap";

const cell = (id: string, cellType: Cell["cellType"] = "code"): Cell => ({ id, cellType, input: id, output: null, status: "idle" });
const cells = [cell("a"), cell("b", "markdown"), cell("r", "raw"), cell("c")];

describe("where the arrow keys go", () => {
  it("goes from a cell's edge to the gap beside it", () => {
    expect(gapBeside(cells, "a", "up")).toEqual({ after: null });
    expect(gapBeside(cells, "b", "up")).toEqual({ after: "a" });
    expect(gapBeside(cells, "b", "down")).toEqual({ after: "b" });
  });

  it("goes from a gap into the cell above at its end, or below at its start", () => {
    expect(fromGap(cells, { after: "a" }, "up")).toEqual({ kind: "cell", cellId: "a", at: "end" });
    expect(fromGap(cells, { after: "a" }, "down")).toEqual({ kind: "cell", cellId: "b", at: "start" });
    expect(fromGap(cells, { after: null }, "down")).toEqual({ kind: "cell", cellId: "a", at: "start" });
  });

  it("stays put at the top and the bottom", () => {
    expect(fromGap(cells, { after: null }, "up")).toBeNull();
    expect(fromGap(cells, { after: "c" }, "down")).toBeNull();
  });

  it("skips over a raw cell, which can't take the cursor", () => {
    expect(fromGap(cells, { after: "b" }, "down")).toEqual({ kind: "gap", point: { after: "r" } });
    expect(fromGap(cells, { after: "r" }, "up")).toEqual({ kind: "gap", point: { after: "b" } });
  });

  it("starts a Markdown cell for text starting with #, a code cell otherwise", () => {
    expect(typeFor("# Notes")).toBe("markdown");
    expect(typeFor("x+1")).toBe("code");
  });
});

describe("the caret in a gap", () => {
  let div: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    nbAddCell.mockClear();
    const store = useNotebookStore.getState();
    store.addTab("nb-1");
    store.setActiveTab("nb-1");
    store.applyBackendState("nb-1", [cell("a"), cell("b")], "notebook_replaced");
    store.setInsertionPoint(null);
    div = document.createElement("div");
    document.body.appendChild(div);
    root = createRoot(div);
    act(() =>
      root.render(
        <I18nProvider i18n={i18n}>
          <CellGap after="a" />
        </I18nProvider>,
      ),
    );
  });

  afterEach(() => {
    act(() => root.unmount());
    div.remove();
  });

  const gap = () => div.querySelector(".cell-gap")!;
  const input = () => div.querySelector<HTMLTextAreaElement>(".cell-gap-input");
  const point = () => {
    const tab = useNotebookStore.getState().notebooks["nb-1"];
    return tab.type === "notebook" ? tab.insertionPoint : undefined;
  };

  function type(text: string) {
    const el = input()!;
    el.value = text;
    act(() => el.dispatchEvent(new InputEvent("input", { bubbles: true })));
  }

  function key(k: string) {
    act(() => input()!.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true })));
  }

  it("places the caret on a click, and takes the keyboard there", () => {
    act(() => gap().dispatchEvent(new MouseEvent("mousedown", { bubbles: true })));
    expect(point()).toEqual({ after: "a" });
    expect(gap().classList.contains("has-caret")).toBe(true);
    expect(document.activeElement).toBe(input());
  });

  it("starts a code cell with the text typed", async () => {
    act(() => useNotebookStore.getState().setInsertionPoint({ after: "a" }));
    type("x");
    await act(async () => {});
    expect(nbAddCell).toHaveBeenCalledWith("code", "x", "a", undefined);
    expect(point()).toBeNull();
  });

  it("starts a Markdown cell for #", async () => {
    act(() => useNotebookStore.getState().setInsertionPoint({ after: "a" }));
    type("#");
    await act(async () => {});
    expect(nbAddCell).toHaveBeenCalledWith("markdown", "#", "a", undefined);
  });

  it("starts an empty code cell on Enter", async () => {
    act(() => useNotebookStore.getState().setInsertionPoint({ after: "a" }));
    key("Enter");
    await act(async () => {});
    expect(nbAddCell).toHaveBeenCalledWith("code", "", "a", undefined);
  });

  it("removes the caret on Escape, adding nothing", () => {
    act(() => useNotebookStore.getState().setInsertionPoint({ after: "a" }));
    key("Escape");
    expect(point()).toBeNull();
    expect(input()).toBeNull();
    expect(nbAddCell).not.toHaveBeenCalled();
  });

  it("moves into the cell below on ArrowDown, at its start", () => {
    act(() => useNotebookStore.getState().setInsertionPoint({ after: "a" }));
    key("ArrowDown");
    const tab = useNotebookStore.getState().notebooks["nb-1"];
    expect(tab.type === "notebook" && tab.pendingCursorMove).toEqual({ cellId: "b", pos: 0 });
    expect(point()).toBeNull();
  });

  it("inserts from the + menu", async () => {
    act(() => div.querySelector<HTMLButtonElement>(".cell-gap-plus")!.click());
    const items = div.querySelectorAll<HTMLButtonElement>(".cell-gap-menu button");
    act(() => items[1].click());
    await act(async () => {});
    expect(nbAddCell).toHaveBeenCalledWith("markdown", "", "a", undefined);
    expect(div.querySelector(".cell-gap-menu")).toBeNull();
  });
});
