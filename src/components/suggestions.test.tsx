// @vitest-environment happy-dom
// The suggestions under a result: quiet buttons inside the cell's output
// area, and none when Settings turns them off
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

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
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), message: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("plotly.js-dist-min", () => ({ default: {} }));
vi.mock("./PlotlyChart", () => ({ PlotlyChart: () => null }));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { Cell } from "./Cell";
import type { Cell as CellModel } from "../types/notebook";

i18n.activate("en");

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
  useNotebookStore.getState().setShowSuggestions(true);
});

const cell: CellModel = {
  id: "c1",
  cellType: "code",
  input: "integrate(x*sin(x), x);",
  status: "success",
  output: {
    textOutput: "", latex: "\\sin x-x\\,\\cos x", plotSvg: null, plotData: null, imagePng: null, error: null,
    errorInfo: null, isError: false, durationMs: 1, outputLabel: "%o1", executionCount: 1, textPlain: null, stale: false,
  },
};

async function render(): Promise<HTMLElement> {
  const div = document.createElement("div");
  document.body.appendChild(div);
  root = createRoot(div);
  await act(async () =>
    root!.render(
      <I18nProvider i18n={i18n}>
        <Cell cell={cell} />
      </I18nProvider>,
    ),
  );
  return div;
}

describe("suggestions under a result", () => {
  it("sit inside the output area, after the output", async () => {
    const row = (await render()).querySelector(".cell-suggestions")!;
    expect(row).not.toBeNull();
    expect(row.parentElement?.classList.contains("cell-output-area")).toBe(true);
    expect(row.previousElementSibling?.classList.contains("cell-output")).toBe(true);
  });

  it("are quiet buttons with an arrow", async () => {
    const buttons = [...(await render()).querySelectorAll(".cell-suggestions button")];
    expect(buttons.map((b) => b.textContent)).toEqual(["Simplify", "Plot"]);
    for (const b of buttons) {
      expect(b.classList.contains("ui-button--quiet")).toBe(true);
      expect(b.querySelector("svg")).not.toBeNull();
    }
    expect(buttons[0].getAttribute("title")).toBe("Simplify the result");
  });

  it("don't show when Settings turns them off", async () => {
    useNotebookStore.getState().setShowSuggestions(false);
    const div = await render();
    expect(div.querySelector(".cell-output-area")).not.toBeNull();
    expect(div.querySelector(".cell-suggestions")).toBeNull();
  });
});
