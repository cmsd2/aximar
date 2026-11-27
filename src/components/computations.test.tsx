import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "@lingui/react";
import { compileMessage } from "@lingui/message-utils/compileMessage";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
// Plotly needs a browser to load
vi.mock("./PlotlyChart", () => ({ PlotlyChart: () => null }));

import { i18n } from "../lib/i18n";
import { CellOutput } from "./CellOutput";
import type { CellOutput as CellOutputType } from "../types/notebook";

function output(fields: Partial<CellOutputType>): CellOutputType {
  return {
    textOutput: "",
    latex: null,
    plotSvg: null,
    plotData: null,
    imagePng: null,
    error: null,
    errorInfo: null,
    isError: false,
    durationMs: 0,
    outputLabel: null,
    executionCount: 1,
    textPlain: null,
    stale: false,
    ...fields,
  };
}

function render(out: CellOutputType): string {
  return renderToStaticMarkup(
    <I18nProvider i18n={i18n}>
      <CellOutput output={out} cellId="c1" />
    </I18nProvider>,
  );
}

/**
 * Spanish, with entries whose English is the same as the cells' output: a
 * catalog that happens to hold "hello" must still leave print text alone
 */
function activateSpanish() {
  const entries: [string, string][] = [
    ["hello", "hola"],
    ["Error", "Fallo"],
    ["x^2/2", "traducido"],
  ];
  i18n.load(
    "es",
    Object.fromEntries(entries.map(([en, es]) => [generateMessageId(en), compileMessage(es)])),
  );
  i18n.activate("es");
}

afterEach(() => {
  i18n.activate("en");
});

describe("computations in Spanish", () => {
  it("show print text and results as Maxima gave them", () => {
    const out = output({ textOutput: "hello\n", textPlain: "x^2/2", latex: "{{x^2}\\over{2}}" });
    const english = render(out);
    activateSpanish();
    const spanish = render(out);
    expect(spanish).toContain("hello");
    expect(spanish).not.toContain("hola");
    expect(spanish).not.toContain("traducido");
    expect(spanish).toBe(english);
  });

  it("show Maxima's own error text as it printed it", () => {
    const error = "Error in sqrt: hello";
    const out = output({ isError: true, error });
    activateSpanish();
    const html = render(out);
    expect(html).toContain(error);
    expect(html).not.toContain("hola");
  });
});
