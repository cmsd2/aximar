import type { Meta, StoryObj } from "@storybook/react-vite";
import { useLingui } from "@lingui/react/macro";
import { Button } from "../../components/ui/Button";
import { Checkbox } from "../../components/ui/Checkbox";
import { Dialog } from "../../components/ui/Dialog";
import { FieldRow } from "../../components/ui/FieldRow";

const meta: Meta = { title: "UI/Dialog", tags: ["themes"] };
export default meta;

function Demo() {
  const { t } = useLingui();
  return (
    <Dialog
      title={t({ message: "Export as LaTeX", comment: "Export as LaTeX: the title of the export window" })}
      onClose={() => {}}
      footer={
        <>
          <Button variant="quiet">{t({ message: "Cancel", comment: "Export as LaTeX button: closes the window without exporting" })}</Button>
          <Button variant="primary">
            {t({ message: "Export", comment: "Export as LaTeX button: asks where to save the .tex file and writes it" })}
          </Button>
        </>
      }
    >
      <FieldRow
        label={t({
          message: "Include code input",
          comment: "Export as LaTeX: checkbox label; when on, the export includes each cell's Maxima input",
        })}
      >
        {({ id }) => <Checkbox id={id} checked onChange={() => {}} />}
      </FieldRow>
    </Dialog>
  );
}

/** A small dialog: title, body and footer, over the overlay */
export const Small: StoryObj = { render: () => <Demo /> };
