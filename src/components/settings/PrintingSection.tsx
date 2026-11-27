import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { AppConfig } from "../../lib/config-client";
import { labelText, SettingRow, type Label, type SectionProps } from "./SettingRow";

const PRINT_FONT_SIZES = [8, 9, 10, 11, 12, 13, 14, 15, 16];
const MARGIN_SIDES: { side: "top" | "bottom" | "left" | "right"; label: Label }[] = [
  { side: "top", label: msg({ message: "Top", comment: "Settings: label of the top print margin field" }) },
  { side: "bottom", label: msg({ message: "Bottom", comment: "Settings: label of the bottom print margin field" }) },
  { side: "left", label: msg({ message: "Left", comment: "Settings: label of the left print margin field" }) },
  { side: "right", label: msg({ message: "Right", comment: "Settings: label of the right print margin field" }) },
];

export function PrintingSection({ config, update }: SectionProps) {
  const { t, i18n } = useLingui();
  return (
    <>
      <SettingRow
        label={t({
          message: "Print font size",
          comment: "Settings: the label of the menu that chooses the size of text when printing",
        })}
        hint={t({ message: "The size of text on paper.", comment: "Settings, Printing section: hint under Print font size" })}
      >
        {({ id, describedBy }) => (
          <select
            id={id}
            aria-describedby={describedBy}
            className="settings-select"
            value={config.print_font_size}
            onChange={(e) => update({ print_font_size: Number(e.target.value) })}
          >
            {PRINT_FONT_SIZES.map((s) => (
              <option key={s} value={s}>
                {s}px
              </option>
            ))}
          </select>
        )}
      </SettingRow>

      <SettingRow
        label={t({
          message: "Print margins (mm)",
          comment: "Settings: the label of the four print margin fields, in millimetres",
        })}
        hint={t({
          message: "The space around each printed page.",
          comment: "Settings, Printing section: hint under Print margins",
        })}
      >
        {({ id, describedBy }) => (
          <div className="settings-margins" id={id} role="group" aria-describedby={describedBy}>
            {MARGIN_SIDES.map(({ side, label }) => {
              const key = `print_margin_${side}` as keyof AppConfig;
              return (
                <label key={side} className="settings-margin-field">
                  <span className="settings-margin-label">{labelText(i18n, label)}</span>
                  <input
                    type="number"
                    className="settings-margin-input"
                    min={0}
                    max={50}
                    value={config[key] as number}
                    onChange={(e) => update({ [key]: Math.max(0, Math.min(50, Number(e.target.value))) })}
                  />
                </label>
              );
            })}
          </div>
        )}
      </SettingRow>
    </>
  );
}
