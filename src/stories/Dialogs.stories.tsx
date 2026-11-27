import type { Meta, StoryObj } from "@storybook/react-vite";
import { SettingsModal, type SectionId } from "../components/SettingsModal";
import { ExportLatexModal } from "../components/ExportLatexModal";
import { TemplateChooser } from "../components/TemplateChooser";
import { CommandPalette } from "../components/CommandPalette";
import { ShortcutsDialog } from "../components/ShortcutsDialog";
import { seedNotebook } from "./store";

const meta: Meta = { title: "Dialogs", parameters: { layout: "fullscreen" } };
export default meta;

const noop = () => {};

export const Settings: StoryObj = {
  loaders: [async () => seedNotebook()],
  render: () => <SettingsModal onClose={noop} onSetVariablesOpen={noop} />,
};

function settingsAt(section: SectionId): StoryObj {
  return {
    loaders: [async () => seedNotebook()],
    render: () => <SettingsModal onClose={noop} onSetVariablesOpen={noop} initialSection={section} />,
  };
}

export const SettingsNotebook = settingsAt("notebook");
export const SettingsMaxima = settingsAt("maxima");
export const SettingsPrinting = settingsAt("printing");
export const SettingsAiAssistants = settingsAt("ai-assistants");
export const SettingsTranslation = settingsAt("translation");

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

/** The palette with commands above the functions, for "run" */
export const CommandPaletteCommands: StoryObj = {
  loaders: [async () => seedNotebook([{ id: "c1", cell_type: "code", input: "x;", output: null, status: "idle" }])],
  render: () => <CommandPalette onClose={noop} initialQuery="run" />,
};

/** Help > Keyboard Shortcuts */
export const KeyboardShortcuts: StoryObj = {
  render: () => <ShortcutsDialog onClose={noop} />,
};
