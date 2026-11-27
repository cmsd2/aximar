import type { Meta, StoryObj } from "@storybook/react-vite";
import { useLingui } from "@lingui/react/macro";
import { Button } from "../../components/ui/Button";
import { IconX } from "../../components/ui/icons";

const meta: Meta = { title: "UI/Button", tags: ["themes"] };
export default meta;

const row = { display: "flex", gap: "var(--space-3)", alignItems: "center", marginBottom: "var(--space-4)" } as const;

function States() {
  const { t } = useLingui();
  const exportLabel = t({ message: "Export", comment: "Export as LaTeX button: asks where to save the .tex file and writes it" });
  const cancel = t({ message: "Cancel", comment: "Export as LaTeX button: closes the window without exporting" });
  const configure = t({
    message: "Configure",
    comment: "Settings button: adds Aximar's MCP server to an AI tool's configuration, such as Claude Code's",
  });
  const close = t({ message: "Close", comment: "Settings button: closes the settings window" });
  return (
    <div>
      <div style={row}>
        <Button variant="primary">{exportLabel}</Button>
        <Button variant="secondary">{configure}</Button>
        <Button variant="quiet">{cancel}</Button>
        <Button variant="quiet" icon={IconX} label={close} />
      </div>
      <div style={row}>
        <Button variant="primary" size="sm">
          {exportLabel}
        </Button>
        <Button variant="secondary" size="sm">
          {configure}
        </Button>
        <Button variant="quiet" size="sm">
          {cancel}
        </Button>
      </div>
      <div style={row}>
        <Button variant="primary" disabled>
          {exportLabel}
        </Button>
        <Button variant="secondary" disabled>
          {configure}
        </Button>
        <Button variant="quiet" disabled>
          {cancel}
        </Button>
      </div>
    </div>
  );
}

/** Primary, secondary and quiet, at both sizes, an icon-only button, and disabled */
export const Variants: StoryObj = { render: () => <States /> };

function FocusedPrimary() {
  const { t } = useLingui();
  return (
    <div style={row}>
      <Button variant="primary" autoFocus>
        {t({ message: "Export", comment: "Export as LaTeX button: asks where to save the .tex file and writes it" })}
      </Button>
    </div>
  );
}

/** A button with keyboard focus */
export const Focused: StoryObj = { render: () => <FocusedPrimary /> };
