import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { activateLanguage, languageName } from "../../lib/i18n";
import { openLicences } from "../../lib/config-client";
import { useLogStore } from "../../store/logStore";
import { Button } from "../ui/Button";
import type { Theme } from "../../store/notebookStore";
import { Checkbox } from "../ui/Checkbox";
import { FieldRow } from "../ui/FieldRow";
import type { Label } from "../ui/label";
import { SegmentedControl } from "../ui/SegmentedControl";
import { Select } from "../ui/Select";
import { HelpLanguageRow } from "./HelpLanguageRow";
import type { SectionProps } from "./types";
import { currentPlatform } from "../../commands/shortcut";

export const THEMES: { value: Theme; label: Label }[] = [
  {
    value: "auto",
    label: msg({ message: "Auto", comment: "Settings: theme button that follows the system's light or dark mode" }),
  },
  { value: "light", label: msg({ message: "Light", comment: "Settings: theme button for the light theme" }) },
  {
    value: "paper",
    label: msg({ message: "Paper", comment: "Settings: theme button for a light theme on warm off-white, like paper" }),
  },
  {
    value: "dark",
    label: msg({ message: "Dark", comment: "Settings: theme button for the dark theme, in neutral greys like the system's own dark mode" }),
  },
  {
    value: "night",
    label: msg({
      message: "Night",
      comment: "Settings: theme button for a second dark theme, tinted violet with pastel colours. A word for night or the night sky",
    }),
  },
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

      <HelpLanguageRow config={config} update={update} />

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

      {currentPlatform() === "linux" && (
        <FieldRow
          divided
          label={t({
            message: "Use the system title bar",
            comment: "Settings, Linux only: the label of the checkbox that keeps the desktop's own title bar and menu bar instead of the tabs in the title bar",
          })}
          hint={t({
            message: "Keep the desktop's title bar and menu bar, with the tabs below them. Takes effect when Aximar restarts.",
            comment: "Settings, General section, Linux only: hint under Use the system title bar",
          })}
        >
          {({ id, describedBy }) => (
            <Checkbox id={id} describedBy={describedBy} checked={config.system_title_bar} onChange={(checked) => update({ system_title_bar: checked })} />
          )}
        </FieldRow>
      )}

      <LicencesRow />
    </>
  );
}

/** Opens the licences of the open-source code and fonts Aximar ships */
function LicencesRow() {
  const { t } = useLingui();
  const addLogEntry = useLogStore((s) => s.addEntry);
  return (
    <FieldRow
      divided
      label={t({ message: "Licences", comment: "Settings, General section: the label of the row that opens the open-source licences" })}
      hint={t({
        message: "The open-source software and fonts Aximar uses, and their licences.",
        comment: "Settings, General section: hint under Licences",
      })}
    >
      {({ id, describedBy }) => (
        <Button
          size="sm"
          id={id}
          aria-describedby={describedBy}
          onClick={() => openLicences().catch((e) => addLogEntry("error", `Failed to open the licences: ${e}`, "settings"))}
        >
          {t({
            message: "Show licences",
            comment: "Settings button: opens a window listing the open-source code and fonts Aximar uses, with their licences",
          })}
        </Button>
      )}
    </FieldRow>
  );
}
