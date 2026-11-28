// @vitest-environment happy-dom
// Next steps for a result, such as "Simplify", live in the + menu of the
// gap under it.  The Settings switch turns them off.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const nbAddCell = vi.fn(async () => ({ cell_id: "new" }));
const executeCell = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (command: string) =>
    command === "get_suggestions"
      ? [
          { label: "Simplify", template: "ratsimp(%);", description: "Simplify the result", position: "after" },
          { label: "Plot", template: "plot2d(%, [x, -5, 5]);", description: "Plot the result", position: "after" },
        ]
      : null,
  ),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn() }));
vi.mock("plotly.js-dist-min", () => ({ default: {} }));
vi.mock("../lib/notebook-commands", () => ({ nbAddCell: (...args: unknown[]) => nbAddCell(...(args as [])) }));
vi.mock("../hooks/useMaxima", () => ({ useMaxima: () => ({ executeCell }) }));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { CellGap } from "./CellGap";
import type { Cell } from "../types/notebook";

i18n.activate("en");

const result = (id: string, executionCount: number): Cell => ({
  id,
  cellType: "code",
  input: "integrate(x*sin(x), x);",
  status: "success",
  output: {
    textOutput: "", latex: "\\sin x-x\\,\\cos x", plotSvg: null, plotData: null, imagePng: null, error: null,
    errorInfo: null, isError: false, durationMs: 1, outputLabel: `%o${executionCount}`, executionCount, textPlain: null, stale: false,
  },
});

let root: Root | undefined;
beforeEach(() => {
  nbAddCell.mockClear();
  executeCell.mockClear();
  const store = useNotebookStore.getState();
  store.addTab("nb-1");
  store.setActiveTab("nb-1");
  store.applyBackendState("nb-1", [result("c1", 1), result("c2", 2)], "notebook_replaced");
  store.setInsertionPoint(null);
});
afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
  useNotebookStore.getState().setShowSuggestions(true);
});

async function gap(after: string): Promise<HTMLElement> {
  const div = document.createElement("div");
  document.body.appendChild(div);
  root = createRoot(div);
  await act(async () =>
    root!.render(
      <I18nProvider i18n={i18n}>
        <CellGap after={after} />
      </I18nProvider>,
    ),
  );
  return div;
}

async function openMenu(div: HTMLElement): Promise<HTMLElement> {
  await act(async () => div.querySelector<HTMLButtonElement>(".cell-gap-plus")!.click());
  return div.querySelector<HTMLElement>(".cell-gap-menu")!;
}

const items = (menu: HTMLElement) => [...menu.querySelectorAll("button")].map((b) => b.textContent?.trim());

describe("next steps in the gap's + menu", () => {
  it("follow the cell types, under a heading naming the result", async () => {
    const menu = await openMenu(await gap("c2"));
    expect(items(menu)).toEqual(["Code⏎", "Markdown#", "Simplify", "Plot"]);
    expect(menu.querySelector(".cell-gap-menu-heading")?.textContent).toBe("From Out [2]");
    expect(menu.querySelector<HTMLElement>(".cell-gap-suggestion")?.title).toBe("Simplify the result");
  });

  it("insert a cell after the result and run it", async () => {
    const menu = await openMenu(await gap("c2"));
    await act(async () => menu.querySelector<HTMLButtonElement>(".cell-gap-suggestion")!.click());
    expect(nbAddCell).toHaveBeenCalledWith("code", "ratsimp(%);", "c2", undefined);
    expect(executeCell).toHaveBeenCalledWith("new", "ratsimp(%);");
    expect(document.querySelector(".cell-gap-menu")).toBeNull();
  });

  it("are offered under every result, not only the latest", async () => {
    const older = await openMenu(await gap("c1"));
    expect(items(older)).toContain("Simplify");
    expect(older.querySelector(".cell-gap-menu-heading")?.textContent).toBe("From Out [1]");
  });

  it("leave out saving a plot, which the output's toolbar does", async () => {
    const { invoke } = await import("@tauri-apps/api/core");
    vi.mocked(invoke).mockImplementationOnce(async () => [
      { label: "Save SVG", template: "", description: "Save the plot", action: "save_svg" },
      { label: "Simplify", template: "ratsimp(%);", description: "Simplify the result", position: "after" },
    ]);
    expect(items(await openMenu(await gap("c2")))).toEqual(["Code⏎", "Markdown#", "Simplify"]);
  });

  it("open from the caret with Tab", async () => {
    useNotebookStore.getState().setInsertionPoint({ after: "c2" });
    const div = await gap("c2");
    const input = div.querySelector<HTMLTextAreaElement>(".cell-gap-input")!;
    await act(async () => {
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Tab", bubbles: true }));
    });
    expect(items(div.querySelector<HTMLElement>(".cell-gap-menu")!)).toContain("Simplify");
    expect(nbAddCell).not.toHaveBeenCalled();
  });

  it("don't show when Settings turns them off", async () => {
    useNotebookStore.getState().setShowSuggestions(false);
    const menu = await openMenu(await gap("c2"));
    expect(items(menu)).toEqual(["Code⏎", "Markdown#"]);
    expect(menu.querySelector(".cell-gap-menu-heading")).toBeNull();
  });
});
