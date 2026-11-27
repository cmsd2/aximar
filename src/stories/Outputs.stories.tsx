import type { Meta, StoryObj } from "@storybook/react-vite";
import { PlotViewer } from "../components/PlotViewer";
import type { SyncCell } from "../lib/notebook-commands";
import plotSvg from "./fixtures/plot2d.svg?raw";
import { StoryNotebook } from "./notebook";
import { seedNotebook } from "./store";
import { WaitFor } from "./wait";

const meta: Meta = { title: "Outputs", tags: ["themes"] };
export default meta;

const output = {
  text_output: "", latex: null, plot_svg: null, plot_data: null, image_png: null, error: null,
  is_error: false, duration_ms: 12, output_label: "%o1", execution_count: 1, text_plain: null,
  stale: false, error_info: null, notes: [],
};


const xs = Array.from({ length: 121 }, (_, i) => -Math.PI + (i * 2 * Math.PI) / 120);
const figure = (a: number, k: number) => ({
  data: [
    { type: "scatter", mode: "lines", name: "a sin(kx)", x: xs, y: xs.map((x) => a * Math.sin(k * x)) },
    { type: "scatter", mode: "lines", name: "cos(x)/2", x: xs, y: xs.map((x) => Math.cos(x) / 2) },
  ],
  layout: { title: { text: "Two waves" }, xaxis: { title: { text: "x" } }, yaxis: { title: { text: "y" } } },
});

const plotly = JSON.stringify(figure(1, 1));
const reactive = JSON.stringify({
  ...figure(1.5, 2),
  _reactive: {
    view_id: "v1",
    signals: [
      { name: "a", lo: 0, hi: 3, value: 1.5, kind: "slider" },
      { name: "k", lo: 1, hi: 5, value: 2, kind: "number" },
      { name: "shift", lo: 0, hi: 1, value: 0, kind: "checkbox" },
      { name: "n", lo: 1, hi: 8, value: 4, kind: "dropdown", choices: [1, 2, 4, 8] },
    ],
  },
});

/** A plot2d result: gnuplot's SVG, as an image */
export const SvgPlot: StoryObj = {
  loaders: [
    async () =>
      seedNotebook([
        {
          id: "s1", cell_type: "code", status: "success",
          input: "plot2d([sin(x), cos(x)/2], [x, -π, π]);",
          output: { ...output, output_label: null, plot_svg: plotSvg },
        },
      ]),
  ],
  render: () => (
    <>
      <StoryNotebook />
      <WaitFor selector=".plot-output img" />
    </>
  ),
};

/** An interactive Plotly chart */
export const PlotlyPlot: StoryObj = {
  loaders: [
    async () =>
      seedNotebook([
        {
          id: "p1", cell_type: "code", status: "success",
          input: "ax_draw2d(explicit(sin(x), x, -π, π), explicit(cos(x)/2, x, -π, π));",
          output: { ...output, output_label: null, plot_data: plotly },
        },
      ]),
  ],
  render: () => (
    <>
      <StoryNotebook />
      <WaitFor selector=".plotly-output .main-svg" />
    </>
  ),
};

/** A chart with reactive controls: a slider, a number field, a checkbox and a choice */
export const ReactiveControls: StoryObj = {
  loaders: [
    async () =>
      seedNotebook([
        {
          id: "r1", cell_type: "code", status: "success",
          input: [
            'a : widget_slider("a", 0, 3)$ k : widget_number("k", 1, 5, 2)$',
            'shift : widget_checkbox("shift", 0)$ n : widget_dropdown("n", [1, 2, 4, 8])$',
            "ax_draw2d(explicit(a*sin(k*x), x, -π, π), explicit(cos(x)/2, x, -π, π));",
          ].join("\n"),
          output: { ...output, output_label: null, plot_data: reactive },
        },
      ]),
  ],
  render: () => (
    <>
      <StoryNotebook />
      <WaitFor selector=".plotly-output .main-svg" />
    </>
  ),
};

/** A chart opened in its own tab, with the export buttons */
export const PlotViewerTab: StoryObj = {
  render: () => (
    <div style={{ height: 560, display: "flex", flexDirection: "column" }}>
      <PlotViewer plotData={plotly} />
      <WaitFor selector=".plotly-output .main-svg" />
    </div>
  ),
};

const markdown: SyncCell[] = [
  {
    id: "m1", cell_type: "markdown", status: "idle", output: null,
    input: [
      "# Fourier series",
      "",
      "A function *f* with period $2\\pi$ has the series",
      "",
      "$$f(x) = \\frac{a_0}{2} + \\sum_{n=1}^{\\infty} \\left(a_n \\cos nx + b_n \\sin nx\\right)$$",
      "",
      "## Steps",
      "",
      "1. Define `f(x)` with `:=`.",
      "2. Compute the coefficients with `integrate`.",
      "3. Compare the partial sums with **plot2d**.",
      "",
      "> The series converges to the midpoint at a jump.",
      "",
      "| n | aₙ | bₙ |",
      "| --- | --- | --- |",
      "| 1 | 0 | 2 |",
      "| 2 | 0 | −1 |",
      "",
      "```maxima",
      "b(n) := integrate(x*sin(n*x), x, -π, π)/π;",
      "```",
      "",
      "See the [Maxima manual](https://maxima.sourceforge.io/docs/manual/) for more.",
    ].join("\n"),
  },
];

/** A rendered Markdown cell: headings, maths, a list, a quote, a table and code */
export const Markdown: StoryObj = {
  loaders: [async () => seedNotebook(markdown)],
  render: () => <StoryNotebook />,
};

const running: SyncCell[] = [
  {
    id: "q1", cell_type: "code", status: "success", silent: true, input: "f(x) := x^2 - 2$",
    output: { ...output, output_label: null, text_output: "" },
  },
  { id: "q2", cell_type: "code", status: "running", input: "find_root(f(x), x, 0, 5);", output: null },
  { id: "q3", cell_type: "code", status: "queued", input: "float(sqrt(2));", output: null },
  {
    id: "q4", cell_type: "code", status: "queued", input: "integrate(f(x), x, 0, 1);",
    output: { ...output, latex: "-{{5}\\over{3}}", text_plain: "-5/3", stale: true },
  },
];

/** Run All part way: a cell running, two queued, one with the output of an earlier run */
export const RunningAndQueued: StoryObj = {
  loaders: [async () => seedNotebook(running)],
  render: () => <StoryNotebook />,
};

const integrated: SyncCell[] = [
  {
    id: "i1", cell_type: "code", status: "success", input: "integrate(x*sin(x), x);",
    output: { ...output, latex: "\\sin x-x\\,\\cos x", text_plain: "sin(x)-x*cos(x)" },
  },
];

/** The gap's + menu: a code or Markdown cell, or a next step for the result above */
export const NextStepsMenu: StoryObj = {
  loaders: [async () => seedNotebook(integrated)],
  render: () => (
    <>
      <StoryNotebook />
      <WaitFor
        selector=".cell-gap-menu .cell-gap-suggestion"
        run={async () => {
          const plus = async (): Promise<HTMLButtonElement> =>
            document.querySelector<HTMLButtonElement>('[data-testid="gap-i1"] .cell-gap-plus') ??
            new Promise((r) => setTimeout(() => r(plus()), 50));
          (await plus()).click();
        }}
      />
    </>
  ),
};

/** The same menu with suggestions turned off in Settings (issue #44): cell types only */
export const SuggestionsOff: StoryObj = {
  parameters: { showSuggestions: false },
  loaders: NextStepsMenu.loaders,
  render: () => (
    <>
      <StoryNotebook />
      <WaitFor
        selector=".cell-gap-menu"
        run={async () => {
          const plus = async (): Promise<HTMLButtonElement> =>
            document.querySelector<HTMLButtonElement>('[data-testid="gap-i1"] .cell-gap-plus') ??
            new Promise((r) => setTimeout(() => r(plus()), 50));
          (await plus()).click();
        }}
      />
    </>
  ),
};

const silent: SyncCell[] = [
  { id: "z1", cell_type: "code", status: "success", silent: true, input: "f(x) := x^2 - 2$", output: { ...output, output_label: null } },
  { id: "z2", cell_type: "code", status: "success", input: "kill(x);", output: { ...output, output_label: null } },
  {
    id: "z3", cell_type: "code", status: "success", silent: true, input: 'print("checked")$',
    output: { ...output, output_label: null, text_output: "checked" },
  },
];

/** A definition ending in $ shows nothing under it; a statement ending in ; that gave nothing says so */
export const SilentAndEmpty: StoryObj = {
  loaders: [async () => seedNotebook(silent)],
  render: () => <StoryNotebook />,
};
