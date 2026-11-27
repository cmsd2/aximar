import type { Meta, StoryObj } from "@storybook/react-vite";
import { Toolbar } from "../components/Toolbar";
import { TabBar } from "../components/TabBar";
import { VariablePanel } from "../components/VariablePanel";
import { ShortcutHints } from "../components/ShortcutHints";
import { seedNotebook } from "./store";
import { useNotebookStore } from "../store/notebookStore";
import { useUiStore } from "../store/uiStore";
import { useLogStore } from "../store/logStore";

const meta: Meta = { title: "Window" };
export default meta;

const output = {
  text_output: "", latex: null, plot_svg: null, plot_data: null, image_png: null, error: null,
  is_error: false, duration_ms: 12, output_label: null, execution_count: 1, text_plain: null,
  stale: false, error_info: null, notes: [],
};

/** Panels closed, the log read, the session idle */
function seedChrome(options: { running?: boolean; panels?: boolean } = {}) {
  seedNotebook(
    [
      { id: "c1", cell_type: "code", input: "f(x) := x^2$", output, status: "success" },
      { id: "c2", cell_type: "code", input: "find_root(f(x) - 2, x, 0, 5);", output: null, status: options.running ? "running" : "idle" },
    ],
    { dirty: true },
  );
  useNotebookStore.getState().setSessionStatusForNotebook("nb-1", options.running ? "Busy" : "Ready");
  useUiStore.setState({ variablesOpen: !!options.panels, docsOpen: !!options.panels });
  const log = useLogStore.getState();
  log.clearLog();
  log.closeWindow();
  useLogStore.setState({ unreadCount: options.panels ? 2 : 0 });
}

function Chrome() {
  return (
    <div>
      <TabBar />
      <Toolbar />
    </div>
  );
}

/** The tab bar and toolbar of a notebook with unsaved changes, idle */
export const ToolbarAndTabs: StoryObj = {
  tags: ["themes"],
  loaders: [async () => seedChrome()],
  render: () => <Chrome />,
};

/** A cell running: Interrupt is enabled, the status says Busy */
export const ToolbarRunning: StoryObj = {
  tags: ["themes"],
  loaders: [async () => seedChrome({ running: true })],
  render: () => <Chrome />,
};

/** The variables and docs panels open, two unread errors in the log */
export const ToolbarPanelsOpen: StoryObj = {
  tags: ["themes"],
  loaders: [async () => seedChrome({ panels: true })],
  render: () => <Chrome />,
};

/** The variables panel */
export const Variables: StoryObj = {
  loaders: [async () => seedNotebook()],
  render: () => <VariablePanel open />,
};

/** The keyboard shortcut hints, as shown while Cmd or Ctrl is held */
export const Shortcuts: StoryObj = {
  render: () => <ShortcutHints held />,
};
