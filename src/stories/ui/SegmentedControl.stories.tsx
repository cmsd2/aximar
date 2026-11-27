import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { SegmentedControl } from "../../components/ui/SegmentedControl";
import { THEMES } from "../../components/settings/GeneralSection";
import { AUTOCOMPLETE_MODES, MARKDOWN_FONTS } from "../../components/settings/NotebookSection";

const meta: Meta = { title: "UI/SegmentedControl", tags: ["themes"] };
export default meta;

function Demo() {
  const [theme, setTheme] = useState("light");
  const [font, setFont] = useState("computer-modern");
  const [mode, setMode] = useState("active-hint");
  const gap = { display: "flex", flexDirection: "column", alignItems: "flex-start", gap: "var(--space-4)" } as const;
  return (
    <div style={gap}>
      <SegmentedControl options={THEMES} value={theme} onChange={setTheme} />
      <SegmentedControl options={MARKDOWN_FONTS} value={font} onChange={setFont} />
      <SegmentedControl options={AUTOCOMPLETE_MODES} value={mode} onChange={setMode} />
      <div style={{ width: 220 }}>
        <SegmentedControl options={MARKDOWN_FONTS} value={font} onChange={setFont} />
      </div>
    </div>
  );
}

/** Labels from Settings; the last group is squeezed into 220px and scrolls rather than wrap */
export const Options: StoryObj = { render: () => <Demo /> };
