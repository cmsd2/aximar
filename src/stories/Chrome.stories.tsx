import type { Meta, StoryObj } from "@storybook/react-vite";
import { Toolbar } from "../components/Toolbar";
import { TabBar } from "../components/TabBar";
import { VariablePanel } from "../components/VariablePanel";
import { ShortcutHints } from "../components/ShortcutHints";
import { seedNotebook } from "./store";

const meta: Meta = { title: "Window" };
export default meta;

const noop = () => {};

/** The toolbar and tab bar of a notebook with unsaved changes */
export const ToolbarAndTabs: StoryObj = {
  loaders: [async () => seedNotebook([], { dirty: true })],
  render: () => (
    <div>
      <TabBar />
      <Toolbar
        onOpenTemplates={noop}
        onOpenSettings={noop}
        variablesOpen
        onToggleVariables={noop}
        logOpen={false}
        onToggleLog={noop}
        logUnreadCount={2}
        docsOpen={false}
        onToggleDocs={noop}
      />
    </div>
  ),
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
