// @vitest-environment happy-dom
// Saving a plot lives in the output's hover toolbar, beside Copy
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "@lingui/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn() }));
vi.mock("plotly.js-dist-min", () => ({ default: {} }));
vi.mock("./PlotlyChart", () => ({ PlotlyChart: () => <div className="plotly-output" /> }));

import { i18n } from "../lib/i18n";
import { CellOutput } from "./CellOutput";
import type { CellOutput as Output } from "../types/notebook";

i18n.activate("en");

const base: Output = {
  textOutput: "", latex: null, plotSvg: null, plotData: null, imagePng: null, error: null,
  errorInfo: null, isError: false, durationMs: 1, outputLabel: null, executionCount: 1, textPlain: null, stale: false,
};

function toolbar(output: Output): string[] {
  const div = document.createElement("div");
  div.innerHTML = renderToStaticMarkup(
    <I18nProvider i18n={i18n}>
      <CellOutput output={output} cellId="c1" />
    </I18nProvider>,
  );
  return [...div.querySelectorAll(".copy-actions button")].map((b) => `${b.textContent}|${b.getAttribute("title")}`);
}

describe("the output's toolbar on a plot", () => {
  it("saves a gnuplot plot as SVG", () => {
    expect(toolbar({ ...base, plotSvg: "<svg></svg>" })).toEqual(["SVG|Save the plot as SVG"]);
  });

  it("saves a Plotly chart as SVG, PNG or JSON", () => {
    expect(toolbar({ ...base, plotData: "{}" })).toEqual([
      "SVG|Save the plot as SVG",
      "PNG|Save the plot as PNG",
      "JSON|Save the plot as JSON",
    ]);
  });

  it("has no save buttons on a result that isn't a plot", () => {
    expect(toolbar({ ...base, latex: "x^2", textPlain: "x^2" }).some((b) => b.startsWith("SVG"))).toBe(false);
  });
});
