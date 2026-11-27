import type { Meta, StoryObj } from "@storybook/react-vite";
import { SettingsModal } from "../components/SettingsModal";
import { ExportLatexModal } from "../components/ExportLatexModal";
import { TemplateChooser } from "../components/TemplateChooser";
import { CommandPalette } from "../components/CommandPalette";
import { seedNotebook } from "./store";

const meta: Meta = { title: "Dialogs", parameters: { layout: "fullscreen" } };
export default meta;

const noop = () => {};

export const Settings: StoryObj = {
  loaders: [async () => seedNotebook()],
  render: () => <SettingsModal onClose={noop} onSetVariablesOpen={noop} />,
};

export const ExportLatex: StoryObj = {
  loaders: [async () => seedNotebook()],
  render: () => <ExportLatexModal onClose={noop} />,
};

export const Templates: StoryObj = {
  render: () => <TemplateChooser onClose={noop} />,
};

export const CommandPaletteOpen: StoryObj = {
  render: () => <CommandPalette onClose={noop} />,
};
