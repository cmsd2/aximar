import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { activateLanguage, languageName } from "../../lib/i18n";
import type { Theme } from "../../store/notebookStore";
import { ChoiceButtons, SettingRow, type Label, type SectionProps } from "./SettingRow";

const THEMES: { value: Theme; label: Label }[] = [
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
      <SettingRow
        label={t({ message: "Language", comment: "Settings: the label of the language picker" })}
        hint={t({
          message: "The language of the window, the menus and Maxima's messages.",
          comment: "Settings, General section: hint under Language",
        })}
      >
        {({ id, describedBy }) => (
          <select
            id={id}
            aria-describedby={describedBy}
            className="settings-select"
            value={config.language}
            onChange={(e) => {
              update({ language: e.target.value });
              void activateLanguage(e.target.value);
            }}
          >
            {config.languages.map((code) => (
              <option key={code} value={code}>
                {languageName(code)}
              </option>
            ))}
          </select>
        )}
      </SettingRow>

      <SettingRow
        label={t({ message: "Theme", comment: "Settings: the label of the theme buttons" })}
        hint={t({
          message: "Auto follows your system's light or dark mode.",
          comment: "Settings, General section: hint under Theme",
        })}
      >
        {(ids) => <ChoiceButtons {...ids} options={THEMES} value={config.theme} onChange={(theme) => update({ theme })} />}
      </SettingRow>

      <SettingRow
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
          <input
            id={id}
            aria-describedby={describedBy}
            type="checkbox"
            className="settings-checkbox"
            checked={config.variables_open}
            onChange={(e) => update({ variables_open: e.target.checked })}
          />
        )}
      </SettingRow>
    </>
  );
}
