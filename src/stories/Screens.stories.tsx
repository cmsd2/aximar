import type { Meta, StoryObj } from "@storybook/react-vite";
import { TabBar } from "../components/TabBar";
import { Toolbar } from "../components/Toolbar";
import { FindBar } from "../components/FindBar";
import { VariablePanel } from "../components/VariablePanel";
import { DocsPanel } from "../components/DocsPanel";
import { LogWindow, StatusBar } from "../components/LogPanel";
import { useLogStore } from "../store/logStore";
import { DropOverlay } from "../components/DropOverlay";
import type { DragState } from "../hooks/useFileDrop";
import { useUiStore } from "../store/uiStore";
import { useNotebookStore } from "../store/notebookStore";
import type { SessionStatus } from "../types/maxima";
import type { SyncCell } from "../lib/notebook-commands";
import welcome from "../../crates/aximar-core/src/notebooks/welcome.json";
import a5Showcase from "./fixtures/a5-showcase.json";
import { fixtures } from "./fixtures";
import { StoryNotebook } from "./notebook";
import { seedNotebook } from "./store";
import { WaitFor } from "./wait";

const meta: Meta = { title: "Screens", tags: ["themes"] };
export default meta;

const noop = () => {};

interface WindowProps {
  variables?: boolean;
  docs?: string;
  /** The window's height in px */
  height?: number;
}

/** The window's layout, as App.tsx builds it, without App's start-up effects */
function Window({ variables = false, docs, height = 760 }: WindowProps) {
  return (
    <div className="app" style={{ height }}>
      <TabBar />
      <Toolbar />
      <FindBar />
      <VariablePanel open={variables} />
      <div className="main-content">
        <StoryNotebook />
        {docs !== undefined && <DocsPanel open functionName={docs} requestId={1} onClose={noop} />}
      </div>
      <LogWindow />
      <StatusBar />
      {docs !== undefined && <WaitFor selector=".docs-function-name" />}
    </div>
  );
}

/** Seed the notebook, the session state and the log */
function seed(cells: SyncCell[], status: SessionStatus, log: [string, "info" | "error", string][] = [], dirty = false) {
  seedNotebook(cells, { dirty });
  // Panels closed; a story opens them with panels()
  useUiStore.setState({ variablesOpen: false, docsOpen: false });
  useNotebookStore.getState().setSessionStatusForNotebook("nb-1", status);
  const store = useLogStore.getState();
  store.clearLog();
  store.closeWindow();
  for (const [source, level, message] of log) store.addEntry(level, message, source);
}

const output = {
  text_output: "", latex: null, plot_svg: null, plot_data: null, image_png: null, error: null,
  is_error: false, duration_ms: 12, output_label: null, execution_count: 1, text_plain: null,
  stale: false, error_info: null, notes: [],
};
const rat = fixtures.notes[0];
const working: SyncCell[] = [
  { id: "m1", cell_type: "markdown", status: "idle", output: null, input: "# Integrals\nCheck a few antiderivatives by differentiating them." },
  {
    id: "c1", cell_type: "code", status: "success", input: "F : integrate(1/(1+x^2), x);",
    output: { ...output, output_label: "%o1", latex: "\\arctan x", text_plain: "atan(x)" },
  },
  {
    id: "c2", cell_type: "code", status: "success", input: "diff(F, x);",
    output: { ...output, output_label: "%o2", execution_count: 2, latex: "{{1}\\over{x^2+1}}", text_plain: "1/(x^2+1)" },
  },
  {
    id: "c3", cell_type: "code", status: "success", input: "rat(0.5*x);",
    output: { ...output, output_label: "%o3", execution_count: 3, latex: "{{x}\\over{2}}", text_plain: "x/2", text_output: rat.text_output, notes: rat.notes },
  },
  {
    id: "c4", cell_type: "code", status: "error", input: "1/0;",
    output: { ...output, is_error: true, execution_count: 4, error: fixtures.explanations[0].error, error_info: fixtures.explanations[0].error_info },
  },
];

/** The whole window mid-session: results, a note, an error, the variables and docs panels */
export const WholeWindow: StoryObj = {
  loaders: [
    async () => {
      seed(working, "Ready", [
        ["session", "info", "Session starting..."],
        ["session", "info", "Session ready"],
      ], true);
      // The toolbar's toggles read the panels from the store
      useUiStore.setState({ variablesOpen: true, docsOpen: true });
    },
  ],
  render: () => <Window variables docs="integrate" />,
};

/**
 * The README's screenshot: the Cayley graph of A5, the app's icon, built
 * with the permutations package and drawn with ax-plots.  The cells and
 * outputs come from running assets/a5-cayley.macnb with `aximar-mcp run`.
 */
export const Showcase: StoryObj = {
  parameters: { cellStyle: "bracket" },
  loaders: [
    async () => {
      seed(a5Showcase as SyncCell[], "Ready", [
        ["session", "info", "Session starting..."],
        ["session", "info", "Session ready"],
      ]);
      useNotebookStore.getState().setNotebookPath("nb-1", "a5-cayley.macnb");
    },
  ],
  render: () => (
    <>
      <Window height={1160} />
      <WaitFor selector=".plotly-output .main-svg" />
    </>
  ),
};

/** The welcome notebook on first launch, before anything has run */
export const FirstRun: StoryObj = {
  loaders: [
    async () => {
      const cells: SyncCell[] = welcome.cells.map((c, i) => ({
        id: `w${i}`,
        cell_type: c.cell_type,
        input: Array.isArray(c.source) ? c.source.join("") : c.source,
        output: null,
        status: "idle",
      }));
      seed(cells, "Starting", [["session", "info", "Session starting..."]]);
    },
  ],
  render: () => <Window />,
};

// What the session reports when the maxima command isn't on the PATH.
// The window also shows a native dialog with install instructions, which
// a story can't show.
const notFound = "Failed to start Maxima: maxima: No such file or directory (os error 2)";

/** Maxima isn't installed: the toolbar's status, the status bar, and an empty notebook */
export const MaximaNotFound: StoryObj = {
  loaders: [
    async () =>
      seed([{ id: "c1", cell_type: "code", input: "", output: null, status: "idle" }], { Error: notFound }, [
        ["session", "info", "Session starting..."],
        ["session", "error", `Session failed: ${notFound}`],
      ]),
  ],
  render: () => <Window />,
};

function dropStory(drag: DragState): StoryObj {
  return {
    loaders: WholeWindow.loaders,
    render: () => (
      <>
        <Window />
        <DropOverlay drag={drag} />
      </>
    ),
  };
}

/** Two notebooks dragged over the window */
export const DropNotebooks = dropStory({ notebooks: 2, others: 0 });
/** A notebook and a file Aximar doesn't open, dragged together */
export const DropMixed = dropStory({ notebooks: 1, others: 1 });
/** Only files that aren't notebooks */
export const DropRefused = dropStory({ notebooks: 0, others: 1 });
