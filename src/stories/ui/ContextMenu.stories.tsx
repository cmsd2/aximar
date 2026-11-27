import type { Meta, StoryObj } from "@storybook/react-vite";
import { ContextMenu } from "../../components/ui/ContextMenu";
import { formatShortcut } from "../../commands/shortcut";

const meta: Meta = { title: "UI/ContextMenu", tags: ["themes"] };
export default meta;

const noop = () => {};

/** A menu at the pointer, in groups, with shortcuts and a disabled item */
export const Groups: StoryObj = {
  render: () => (
    <div style={{ height: 300 }}>
      <ContextMenu
        x={40}
        y={30}
        onClose={noop}
        items={[
          { id: "run", label: "Run", shortcut: formatShortcut("Shift+Enter"), onChoose: noop },
          { id: "stay", label: "Run and Stay", shortcut: formatShortcut("Mod+Enter"), onChoose: noop },
          { id: "up", label: "Move Up", shortcut: formatShortcut("Mod+Shift+ArrowUp"), disabled: true, separatorBefore: true, onChoose: noop },
          { id: "down", label: "Move Down", shortcut: formatShortcut("Mod+Shift+ArrowDown"), onChoose: noop },
          { id: "delete", label: "Delete Cell", shortcut: formatShortcut("Mod+Shift+Backspace"), separatorBefore: true, onChoose: noop },
        ]}
      />
    </div>
  ),
};
