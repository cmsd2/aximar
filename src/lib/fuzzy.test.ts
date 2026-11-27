import { describe, expect, it } from "vitest";
import { formatter } from "@lingui/format-po-gettext";
import { withoutFuzzy } from "./po-format";

const po = `msgid ""
msgstr ""
"Language: es\\n"
"Content-Type: text/plain; charset=UTF-8\\n"
"Plural-Forms: nplurals=2; plural=(n != 1);\\n"

msgid "Run all"
msgstr "Ejecutar todo"

#, fuzzy
#| msgid "Run every cell"
msgid "Run all cells"
msgstr "Ejecutar cada celda"
`;

describe("the window's catalogs", () => {
  it("leave fuzzy entries untranslated until a translator reviews them", async () => {
    const catalog = await withoutFuzzy(formatter({ lineNumbers: false })).parse(po, {
      locale: "es",
      sourceLocale: "en",
      filename: "es.po",
    });
    const byMessage = Object.fromEntries(
      Object.values(catalog ?? {}).map((m) => [m.message, m.translation] as const),
    );
    expect(byMessage["Run all"]).toBe("Ejecutar todo");
    expect(byMessage["Run all cells"]).toBe("");
  });
});
