import type { Meta, StoryObj } from "@storybook/react-vite";
import { Cell } from "../components/Cell";
import { TrustBanner } from "../components/TrustBanner";
import { useNotebookStore } from "../store/notebookStore";
import type { SyncCell } from "../lib/notebook-commands";
import { fixtures } from "./fixtures";
import { seedNotebook } from "./store";

const meta: Meta = { title: "Cells" };
export default meta;

const output = {
  text_output: "", latex: null, plot_svg: null, plot_data: null, image_png: null, error: null,
  is_error: false, duration_ms: 12, output_label: "%o1", execution_count: 1, text_plain: null,
  stale: false, error_info: null, notes: [],
};
const rat = fixtures.notes[0];
const cells: SyncCell[] = [
  { id: "m1", cell_type: "markdown", input: "# Notes\nSome *text*.", output: null, status: "idle" },
  {
    id: "c1", cell_type: "code", input: "rat(0.5*x);", status: "success",
    output: { ...output, latex: "{{x}\\over{2}}", text_plain: "x/2", text_output: rat.text_output, notes: rat.notes },
  },
  {
    id: "c2", cell_type: "code", input: "1/0;", status: "error",
    output: { ...output, is_error: true, output_label: null, error: fixtures.explanations[0].error, error_info: fixtures.explanations[0].error_info },
  },
  { id: "c3", cell_type: "code", input: "", output: null, status: "idle" },
];

function Cells() {
  const tab = useNotebookStore((s) => s.notebooks["nb-1"]);
  if (!tab || tab.type !== "notebook") return null;
  return (
    <div>
      {tab.cells.map((cell) => (
        <Cell key={cell.id} cell={cell} />
      ))}
    </div>
  );
}

/** Code and Markdown cells: a result with a note, an error, an empty cell */
export const Notebook: StoryObj = {
  loaders: [async () => seedNotebook(cells)],
  render: () => <Cells />,
};

/** The banner over an untrusted notebook whose cell calls a blocked function */
export const TrustBannerShown: StoryObj = {
  loaders: [async () => seedNotebook([], { trusted: false, pendingTrust: ["system", "load"] })],
  render: () => <TrustBanner />,
};

const printed: SyncCell[] = [
  {
    id: "p1", cell_type: "code", status: "success",
    input: 'print("before")$ tex(x^2)$ print("a =", x^2)$\nfor n:1 thru 2 do (print("n =", n, ":"), tex(n/(n+1)))$ print("after")$',
    output: { ...output, output_label: null, text_output: "before\n$$x^2$$\na = x^2\nn = 1 :\n$${{1}\\over{2}}$$\nn = 2 :\n$${{2}\\over{3}}$$\nafter" },
  },
  {
    id: "p2", cell_type: "code", status: "success",
    input: 'print("label:")$ tex(sqrt(x))$ tex(integrate(1/(1+x^2),x))$ print("tail")$ x^2/2;',
    output: { ...output, latex: "{{x^2}\\over{2}}", text_plain: "x^2/2", text_output: "label:\n$$\\sqrt{x}$$\n$$\\arctan x$$\ntail" },
  },
  {
    id: "p3", cell_type: "code", status: "success",
    input: 'print("plain text only")$ print("second line")$ x;',
    output: { ...output, latex: "x", text_plain: "x", text_output: "plain text only\nsecond line" },
  },
];

/** print() and tex() output mixed, with and without a result after it */
export const PrintAndTex: StoryObj = {
  loaders: [async () => seedNotebook(printed)],
  render: () => <Cells />,
};
