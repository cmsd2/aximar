import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { activateLanguage, languageName } from "../../lib/i18n";
import type { Theme } from "../../store/notebookStore";
import { Checkbox } from "../ui/Checkbox";
import { FieldRow } from "../ui/FieldRow";
import type { Label } from "../ui/label";
import { SegmentedControl } from "../ui/SegmentedControl";
import { Select } from "../ui/Select";
import type { SectionProps } from "./types";

export const THEMES: { value: Theme; label: Label }[] = [
  {
    value: "auto",
    label: msg({ message: "Auto", comment: "Settings: theme button that follows the system's light or dark mode" }),
  },
  { value: "light", label: msg({ message: "Light", comment: "Settings: theme button for the light theme" }) },
  { value: "dark", label: msg({ message: "Dark", comment: "Settings: theme button for the dark theme" }) },
];

export function GeneralSection({ config, update }: SectionProps) {
  const { t } = useLingui();
  return (
    <>
      <FieldRow
        divided
        label={t({ message: "Language", comment: "Settings: the label of the language picker" })}
        hint={t({
          message: "The language of the window, the menus and Maxima's messages.",
          comment: "Settings, General section: hint under Language",
        })}
      >
        {({ id, describedBy }) => (
          <Select
            id={id}
            describedBy={describedBy}
            value={config.language}
            options={config.languages.map((code) => ({ value: code, label: languageName(code) }))}
            onChange={(language) => {
              update({ language });
              void activateLanguage(language);
            }}
          />
        )}
      </FieldRow>

      <FieldRow
        divided
        label={t({ message: "Theme", comment: "Settings: the label of the theme buttons" })}
        hint={t({
          message: "Auto follows your system's light or dark mode.",
          comment: "Settings, General section: hint under Theme",
        })}
      >
        {(ids) => <SegmentedControl {...ids} options={THEMES} value={config.theme} onChange={(theme) => update({ theme })} />}
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
        {({ id, describedBy }) => (
          <Checkbox id={id} describedBy={describedBy} checked={config.variables_open} onChange={(checked) => update({ variables_open: checked })} />
        )}
      </FieldRow>
    </>
  );
}
