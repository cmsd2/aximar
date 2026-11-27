import { describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "@lingui/react";
import { i18n } from "../lib/i18n";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
// Plotly needs a browser to load
vi.mock("./PlotlyChart", () => ({ PlotlyChart: () => null }));

import { CellOutput } from "./CellOutput";
import { RawCell } from "./RawCell";
import type { CellOutput as CellOutputType } from "../types/notebook";

function render(node: ReactNode): string {
  return renderToStaticMarkup(<I18nProvider i18n={i18n}>{node}</I18nProvider>);
}

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

describe("a result with only its one-line form", () => {
  it("shows the one-line form", () => {
    const html = render(<CellOutput output={output({ textPlain: "x^2/2" })} cellId="c1" />);
    expect(html).toContain("x^2/2");
    expect(html).not.toContain("(no output)");
  });

  it("is shown as LaTeX when it has LaTeX", () => {
    const html = render(
      <CellOutput output={output({ textPlain: "x^2/2", latex: "{{x^2}\\over{2}}" })} cellId="c1" />,
    );
    expect(html).toContain("katex");
    expect(html).not.toContain('<div class="text-output">x^2/2</div>');
  });
});

describe("a raw cell", () => {
  const cell = { id: "r1", cellType: "raw" as const, input: "load(\"x\")$", output: null, status: "idle" as const };

  it("shows its text read-only, with no run button", () => {
    const html = render(<RawCell cell={cell} />);
    expect(html).toContain("<pre class=\"raw-cell-text\">load(&quot;x&quot;)$</pre>");
    expect(html).not.toContain("run-btn");
    expect(html).not.toContain("<textarea");
    expect(html).not.toContain("cm-content");
  });
});

describe("an error with Maxima's message in the user's language", () => {
  const english = "diff: wrong number of arguments.\n -- an error. To debug this try: debugmode(true);";
  const spanish = "diff: número de argumentos equivocados.";

  it("shows the translation, with the English one click away", () => {
    const html = render(
      <CellOutput output={output({ isError: true, error: english, errorTranslated: spanish })} cellId="c1" />,
    );
    expect(html).toContain(spanish);
    expect(html).toContain("Show original");
    expect(html).not.toContain("wrong number of arguments");
  });

  it("shows the translation above Aximar's explanation", () => {
    const errorInfo = {
      title: "Wrong Argument Count",
      explanation: "diff was called with the wrong number of arguments.",
      suggestion: null,
      example: null,
      did_you_mean: [],
      correct_signatures: [],
    };
    const html = render(
      <CellOutput output={output({ isError: true, error: english, errorInfo, errorTranslated: spanish })} cellId="c1" />,
    );
    expect(html.indexOf(spanish)).toBeGreaterThan(html.indexOf("Wrong Argument Count"));
    expect(html.indexOf(spanish)).toBeLessThan(html.indexOf("diff was called"));
  });

  it("shows the English alone when there's no translation", () => {
    const html = render(<CellOutput output={output({ isError: true, error: english })} cellId="c1" />);
    expect(html).toContain("wrong number of arguments");
    expect(html).not.toContain("Show original");
  });
});

describe("an error with suggestions", () => {
  const errorInfo = {
    title: "Unknown Function",
    explanation: "intgrate is not a known function.",
    suggestion: null,
    example: "integrate(x^2, x)",
    did_you_mean: ["integrate", "intopois"],
    correct_signatures: ["integrate(expr, x)"],
  };

  it("shows each suggested name as a button inside the question", () => {
    const html = render(
      <CellOutput output={output({ isError: true, error: "intgrate(x)", errorInfo })} cellId="c1" />,
    );
    expect(html).toContain(
      'Did you mean: <span><button class="dym-link">integrate</button></span><span>, <button class="dym-link">intopois</button></span>?',
    );
  });

  it("shows the usage and a clickable example", () => {
    const html = render(
      <CellOutput output={output({ isError: true, error: "intgrate(x)", errorInfo })} cellId="c1" />,
    );
    expect(html).toContain("Correct usage:");
    expect(html).toContain(
      'Example: <code class="enhanced-error-example-code" title="Click to use this example">integrate(x^2, x)</code>',
    );
  });
});
