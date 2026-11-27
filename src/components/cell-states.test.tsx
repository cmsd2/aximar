// @vitest-environment happy-dom
// A cell's label and state class while it runs, waits in a Run All, or has run
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(async () => null) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), message: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("./PlotlyChart", () => ({ PlotlyChart: () => null }));

import { i18n } from "../lib/i18n";
import { Cell } from "./Cell";
import type { Cell as CellModel, CellStatus } from "../types/notebook";

i18n.activate("en");

let root: Root | undefined;
afterEach(() => {
  act(() => root?.unmount());
  root = undefined;
});

function render(status: CellStatus, executionCount: number | null = null): HTMLElement {
  const cell: CellModel = {
    id: "c1",
    cellType: "code",
    input: "find_root(f(x), x, 0, 5);",
    status,
    output: executionCount
      ? {
          textOutput: "", latex: "1.414", plotSvg: null, plotData: null, imagePng: null, error: null,
          errorInfo: null, isError: false, durationMs: 1, outputLabel: null, executionCount, textPlain: null, stale: false,
        }
      : null,
  };
  const div = document.createElement("div");
  document.body.appendChild(div);
  root = createRoot(div);
  act(() =>
    root!.render(
      <I18nProvider i18n={i18n}>
        <Cell cell={cell} />
      </I18nProvider>,
    ),
  );
  return div;
}

const label = (div: HTMLElement) => div.querySelector(".cell-input-area .cell-indicator")?.textContent;

describe("a cell's run state", () => {
  it("reads In [*] while it runs", () => {
    const div = render("running");
    expect(label(div)).toBe("In [*]");
    expect(div.querySelector(".cell")?.classList.contains("running")).toBe(true);
  });

  it("reads In [·] while it waits in a Run All", () => {
    const div = render("queued");
    expect(label(div)).toBe("In [·]");
    expect(div.querySelector(".cell")?.classList.contains("queued")).toBe(true);
  });

  it("reads In [n] once it has run", () => {
    expect(label(render("success", 3))).toBe("In [3]");
  });
});
