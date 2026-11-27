import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { Checkbox } from "../../components/ui/Checkbox";
import { FieldRow } from "../../components/ui/FieldRow";
import { Select } from "../../components/ui/Select";
import { TextField } from "../../components/ui/TextField";

const meta: Meta = { title: "UI/Fields", tags: ["themes"] };
export default meta;

function Demo() {
  const { t } = useLingui();
  const [open, setOpen] = useState(true);
  const [size, setSize] = useState(14);
  return (
    <div style={{ maxWidth: 560 }}>
      <FieldRow
        divided
        label={t({ message: "Maxima path", comment: "Settings: the label of the field for the location of the maxima program" })}
        hint={t({ message: "Leave it empty and Aximar finds Maxima itself.", comment: "Settings, Maxima section: hint under Maxima path" })}
      >
        {(ids) => (
          <TextField
            {...ids}
            mono
            value=""
            placeholder={t({
              message: "Auto-detect",
              comment: "Settings: placeholder in the Maxima path field; when empty, Aximar finds maxima itself",
            })}
          />
        )}
      </FieldRow>
      <FieldRow
        divided
        label={t({ message: "Font size", comment: "Settings: the label of the menu that chooses the size of code text" })}
        hint={t({ message: "The size of code and output text.", comment: "Settings, Notebook section: hint under Font size" })}
      >
        {(ids) => (
          <Select {...ids} value={size} onChange={setSize} options={[12, 13, 14, 15, 16].map((s) => ({ value: s, label: `${s}px` }))} />
        )}
      </FieldRow>
      <FieldRow
        divided
        label={t({
          message: "Variable panel open by default",
          comment: "Settings: the label of the checkbox that opens the variable panel when Aximar starts",
        })}
        hint={t({
          message: "Show the variable panel when Aximar starts.",
          comment: "Settings, General section: hint under Variable panel open by default",
        })}
      >
        {(ids) => <Checkbox {...ids} checked={open} onChange={setOpen} />}
      </FieldRow>
      <FieldRow
        divided
        wide
        label={t({ message: "MCP token", comment: "Settings: the label of the secret that AI tools send to Aximar's MCP server" })}
        hint={t({
          message: "AI assistants send this secret with each request. Reconfigure them after a change.",
          comment: "Settings, AI assistants section: hint under MCP token. Reconfigure is the button beside each AI assistant",
        })}
      >
        {(ids) => <TextField {...ids} mono readOnly value="0123456789abcdef" />}
      </FieldRow>
      <FieldRow
        label={t({
          message: "Include plots",
          comment: "Export as LaTeX: checkbox label; when on, the export includes plots as image files",
        })}
      >
        {({ id }) => <Checkbox id={id} checked onChange={() => {}} />}
      </FieldRow>
    </div>
  );
}

/** FieldRow with a text field, a menu and checkboxes; divided as in Settings, wide for long controls, and plain as in a dialog */
export const Rows: StoryObj = { render: () => <Demo /> };
