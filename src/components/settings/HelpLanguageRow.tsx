import { useEffect, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { docsLanguages } from "../../lib/catalog-client";
import { languageName } from "../../lib/i18n";
import type { DocsLanguages } from "../../types/catalog";
import { FieldRow } from "../ui/FieldRow";
import { Select } from "../ui/Select";
import type { SectionProps } from "./types";

/** "Same as Aximar's language" in the menu's value; the config keeps it as null */
export const SAME_AS_APP = "same";

/**
 * The language of the help in the docs panel: the same as Aximar's, or one
 * of the languages with documentation, each by its own name
 */
export function HelpLanguageRow({ config, update }: SectionProps) {
  const { t } = useLingui();
  const [info, setInfo] = useState<DocsLanguages | null>(null);

  useEffect(() => {
    let current = true;
    docsLanguages()
      .then((i) => current && setInfo(i))
      .catch(() => current && setInfo(null));
    return () => {
      current = false;
    };
  }, []);

  const options = [
    {
      value: SAME_AS_APP,
      label: t({
        message: "Same as Aximar's language",
        comment: "Settings: the first choice for Help language; the help follows the Language setting above it. max-length:32",
      }),
    },
    ...(info?.languages ?? []).map((l) => ({ value: l.tag, label: languageName(l.tag) })),
  ];

  return (
    <FieldRow
      divided
      label={t({ message: "Help language", comment: "Settings: the label of the menu that chooses the language of the docs panel's help" })}
      hint={t({
        message: "The language of function help in the docs panel. Where a function has no help in it, the English shows.",
        comment: "Settings, General section: hint under Help language",
      })}
    >
      {({ id, describedBy }) => (
        <Select
          id={id}
          describedBy={describedBy}
          value={config.docs_language ?? SAME_AS_APP}
          options={options}
          onChange={(value) => update({ docs_language: value === SAME_AS_APP ? null : value })}
        />
      )}
    </FieldRow>
  );
}
