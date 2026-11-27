import type { Meta, StoryObj } from "@storybook/react-vite";
import { EditorView } from "@codemirror/view";
import { startCompletion } from "@codemirror/autocomplete";
import { FindBar } from "../components/FindBar";
import { useFindStore } from "../store/findStore";
import type { SyncCell } from "../lib/notebook-commands";
import { StoryNotebook } from "./notebook";
import { seedNotebook } from "./store";
import { WaitFor } from "./wait";

const meta: Meta = { title: "Editor", tags: ["themes"] };
export default meta;


const output = {
  text_output: "", latex: null, plot_svg: null, plot_data: null, image_png: null, error: null,
  is_error: false, duration_ms: 12, output_label: "%o1", execution_count: 1, text_plain: null,
  stale: false, error_info: null, notes: [],
};

const findCells: SyncCell[] = [
  { id: "m1", cell_type: "markdown", input: "## Taylor series\nExpand *f* about *x* = 0.", output: null, status: "idle" },
  {
    id: "c1", cell_type: "code", input: "f(x) := exp(x)*sin(x)$\ntaylor(f(x), x, 0, 4);", status: "success",
    output: { ...output, latex: "x+x^2+{{x^3}\\over{3}}+\\cdots", text_plain: "x+x^2+x^3/3+..." },
  },
];

/** Find and Replace open over a notebook, on the second of several matches */
export const FindAndReplace: StoryObj = {
  loaders: [
    async () => {
      seedNotebook(findCells);
      const find = useFindStore.getState();
      find.close();
      find.open(true);
      find.setQuery("x");
      find.setReplacement("t");
    },
  ],
  render: () => (
    <div>
      <FindBar />
      <StoryNotebook />
      <WaitFor selector=".find-bar" run={async () => useFindStore.getState().goToNextMatch()} />
    </div>
  ),
};

/** The one editor on the page, once it has mounted */
async function editor(): Promise<EditorView> {
  for (;;) {
    const dom = document.querySelector<HTMLElement>("#storybook-root .cm-editor");
    const view = dom && EditorView.findFromDOM(dom);
    if (view) return view;
    await new Promise((r) => setTimeout(r, 20));
  }
}

/** Type `text` at the end of the editor, as a keystroke would */
async function typeAtEnd(text: string): Promise<EditorView> {
  const view = await editor();
  view.focus();
  const end = view.state.doc.length;
  view.dispatch({ changes: { from: end, insert: text }, selection: { anchor: end + text.length }, userEvent: "input.type" });
  return view;
}

function oneCell(input: string): SyncCell[] {
  return [{ id: "c1", cell_type: "code", input, output: null, status: "idle" }];
}

/** The completion list while typing a function name, with one from a package */
export const Completion: StoryObj = {
  loaders: [async () => seedNotebook(oneCell("in"))],
  render: () => (
    <div style={{ minHeight: 360 }}>
      <StoryNotebook />
      <WaitFor
        selector=".cm-tooltip-autocomplete"
        run={async () => {
          const view = await typeAtEnd("t");
          startCompletion(view);
        }}
      />
    </div>
  ),
};

/** Argument help after typing a function's opening bracket */
export const ArgumentHelp: StoryObj = {
  loaders: [async () => seedNotebook(oneCell("integrate"))],
  render: () => (
    <div style={{ minHeight: 240 }}>
      <StoryNotebook />
      <WaitFor selector=".signature-hint" run={async () => void (await typeAtEnd("("))} />
    </div>
  ),
};
