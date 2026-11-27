import { useLingui } from "@lingui/react/macro";
import { languageName } from "../../lib/i18n";
import { Select } from "../ui/Select";
import type { DocsLanguages } from "../../types/catalog";

/** "Same as Aximar" in the menu's value */
export const SAME_AS_APP = "same";

/**
 * The docs panel's choice of documentation language: "Same as Aximar",
 * or one of the languages with documentation, each by its own name
 */
export function DocsLanguageMenu({
  info,
  onChange,
}: {
  info: DocsLanguages;
  /** A language tag, or null to follow the app's language */
  onChange: (language: string | null) => void;
}) {
  const { t } = useLingui();
  const options = [
    {
      value: SAME_AS_APP,
      label: t({
        message: "Same as Aximar",
        comment: "Docs panel: documentation language menu item; the help follows the app's language setting. max-length:24",
      }),
    },
    ...info.languages.map((l) => ({ value: l.tag, label: languageName(l.tag) })),
  ];
  return (
    <span
      className="docs-language-menu"
      title={t({
        message: "Documentation language",
        comment: "Docs panel: tooltip of the menu that chooses the language of the help",
      })}
    >
      <Select
        value={info.choice ?? SAME_AS_APP}
        options={options}
        onChange={(value) => onChange(value === SAME_AS_APP ? null : value)}
      />
    </span>
  );
}
