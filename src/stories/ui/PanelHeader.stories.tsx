import type { Meta, StoryObj } from "@storybook/react-vite";
import { PanelHeader } from "../../components/ui/PanelHeader";
import { Button } from "../../components/ui/Button";

const meta: Meta = { title: "UI/PanelHeader", tags: ["themes"] };
export default meta;

const noop = () => {};

/** A title, an action and the close button */
export const TitleAndActions: StoryObj = {
  render: () => (
    <div style={{ maxWidth: 520, border: "1px solid var(--border-color)" }}>
      <PanelHeader
        title="Variables"
        actions={<Button size="sm" variant="secondary">Kill All</Button>}
        onClose={noop}
        closeLabel="Close variables"
      />
    </div>
  ),
};

/** Controls on the left instead of a title */
export const Controls: StoryObj = {
  render: () => (
    <div style={{ maxWidth: 520, border: "1px solid var(--border-color)" }}>
      <PanelHeader onClose={noop} closeLabel="Close docs">
        <input className="docs-search-input" placeholder="Search functions..." />
      </PanelHeader>
    </div>
  ),
};
