import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { AutocompleteMode, CellStyle } from "../../store/notebookStore";
import { ChoiceButtons, SettingRow, type Label, type SectionProps } from "./SettingRow";

const FONT_SIZES = [12, 13, 14, 15, 16];

const CELL_STYLES: { value: CellStyle; label: Label }[] = [
  {
    value: "card",
    label: msg({ message: "Card", comment: "Settings: cell style button that draws each cell as a boxed card" }),
  },
  {
    value: "bracket",
    label: msg({ message: "Bracket", comment: "Settings: cell style button that marks each cell with a bracket in the margin" }),
  },
];
const AUTOCOMPLETE_MODES: { value: AutocompleteMode; label: Label }[] = [
  {
    value: "hint",
    label: msg({ message: "Hint", comment: "Settings: argument help button; shows a function's arguments as a hint while typing" }),
  },
  {
    value: "snippet",
    label: msg({
      message: "Snippet",
      comment: "Settings: argument help button; completing a function inserts its arguments as placeholders to fill in",
    }),
  },
  {
    value: "active-hint",
    label: msg({ message: "Active hint", comment: "Settings: argument help button; the hint follows the argument under the cursor" }),
  },
];
const MARKDOWN_FONTS: { value: string; label: Label }[] = [
  {
    value: "sans-serif",
    label: msg({ message: "Sans-serif", comment: "Settings: Markdown font button for the system sans-serif font" }),
  },
  { value: "serif", label: msg({ message: "Serif", comment: "Settings: Markdown font button for the system serif font" }) },
  { value: "computer-modern", label: "Computer Modern" },
  { value: "mono", label: msg({ message: "Mono", comment: "Settings: Markdown font button for the monospace font" }) },
];
const MARKDOWN_INDENTS: { value: string; label: Label }[] = [
  {
    value: "flush",
    label: msg({ message: "Flush", comment: "Settings: Markdown indent button; text cells start at the left edge" }),
  },
  {
    value: "aligned",
    label: msg({ message: "Aligned", comment: "Settings: Markdown indent button; text cells line up with the code in code cells" }),
  },
];

export function NotebookSection({ config, update }: SectionProps) {
  const { t } = useLingui();
  return (
    <>
      <SettingRow
        label={t({ message: "Font size", comment: "Settings: the label of the menu that chooses the size of code text" })}
        hint={t({ message: "The size of code and output text.", comment: "Settings, Notebook section: hint under Font size" })}
      >
        {({ id, describedBy }) => (
          <select
            id={id}
            aria-describedby={describedBy}
            className="settings-select"
            value={config.font_size}
            onChange={(e) => update({ font_size: Number(e.target.value) })}
          >
            {FONT_SIZES.map((s) => (
              <option key={s} value={s}>
                {s}px
              </option>
            ))}
          </select>
        )}
      </SettingRow>

      <SettingRow
        label={t({ message: "Cell style", comment: "Settings: the label of the buttons that choose how cells are drawn" })}
        hint={t({
          message: "Card draws each cell in a box. Bracket marks it in the margin.",
          comment: "Settings, Notebook section: hint under Cell style. Card and Bracket are the buttons' labels",
        })}
      >
        {(ids) => (
          <ChoiceButtons {...ids} options={CELL_STYLES} value={config.cell_style} onChange={(cell_style) => update({ cell_style })} />
        )}
      </SettingRow>

      <SettingRow
        label={t({
          message: "Argument help",
          comment: "Settings: the label of the buttons that choose how function arguments are shown while typing",
        })}
        hint={t({
          message: "Hint shows the arguments, Snippet inserts them, Active hint follows the cursor.",
          comment: "Settings, Notebook section: hint under Argument help. Hint, Snippet and Active hint are the buttons' labels",
        })}
      >
        {(ids) => (
          <ChoiceButtons
            {...ids}
            options={AUTOCOMPLETE_MODES}
            value={config.autocomplete_mode}
            onChange={(autocomplete_mode) => update({ autocomplete_mode })}
          />
        )}
      </SettingRow>

      <SettingRow
        label={t({ message: "Markdown font", comment: "Settings: the label of the buttons that choose the font of text cells" })}
        hint={t({ message: "The font of text cells.", comment: "Settings, Notebook section: hint under Markdown font" })}
      >
        {(ids) => (
          <ChoiceButtons
            {...ids}
            options={MARKDOWN_FONTS}
            value={config.markdown_font}
            onChange={(markdown_font) => update({ markdown_font })}
          />
        )}
      </SettingRow>

      <SettingRow
        label={t({
          message: "Markdown indent",
          comment: "Settings: the label of the buttons that choose how far text cells are indented",
        })}
        hint={t({
          message: "Aligned lines text cells up with the code in code cells.",
          comment: "Settings, Notebook section: hint under Markdown indent. Aligned is a button's label",
        })}
      >
        {(ids) => (
          <ChoiceButtons
            {...ids}
            options={MARKDOWN_INDENTS}
            value={config.markdown_indent}
            onChange={(markdown_indent) => update({ markdown_indent })}
          />
        )}
      </SettingRow>
    </>
  );
}
