import { afterEach, describe, expect, it } from "vitest";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";
import { i18n, setLanguageOverride } from "./i18n";
import { applyPreview, compilePreview, type PreviewCatalog } from "./translation-preview";

const icu = "{count, plural, one {# cell} other {# cells}}";
const catalog: PreviewCatalog = {
  path: "/work/es.po",
  language: "es",
  plural_forms: "nplurals=2; plural=(n != 1);",
  entries: [
    { context: null, msgid: "Show raw error", msgid_plural: null, msgstr: "Mostrar error original", msgstr_plural: [], comments: [] },
    {
      context: "error.arg_count.explanation",
      msgid: "{func} is defined as {definition}, but was called with {found}.",
      msgid_plural: null,
      msgstr: "{func} se definió como {definition}, pero se llamó con {found}.",
      msgstr_plural: [],
      comments: [],
    },
    {
      context: null,
      msgid: "# cell",
      msgid_plural: "# cells",
      msgstr: "",
      msgstr_plural: ["# celda", "# celdas"],
      comments: [`js-lingui:icu=${encodeURIComponent(icu).replace(/%20/g, "+")}&pluralize_on=count`],
    },
  ],
};

afterEach(async () => {
  await setLanguageOverride(false, "en");
});

describe("a loaded catalog", () => {
  it("compiles under the IDs the window looks up", () => {
    const messages = compilePreview(catalog);
    expect(Object.keys(messages)).toHaveLength(3);
    expect(messages).toHaveProperty(generateMessageId("Show raw error"));
    expect(messages).toHaveProperty(
      generateMessageId(catalog.entries[1].msgid, "error.arg_count.explanation"),
    );
    expect(messages).toHaveProperty(generateMessageId(icu));
  });

  it("shows its translations, with parameters and plurals, and English for the rest", async () => {
    await applyPreview(catalog);
    expect(i18n.locale).toBe("es");
    expect(i18n._({ id: generateMessageId("Show raw error"), message: "Show raw error" })).toBe(
      "Mostrar error original",
    );
    const template = catalog.entries[1].msgid;
    expect(
      i18n._({
        id: generateMessageId(template, "error.arg_count.explanation"),
        message: template,
        values: { func: "f", definition: "f(x)", found: "[1,2]" },
      }),
    ).toBe("f se definió como f(x), pero se llamó con [1,2].");
    expect(i18n._({ id: generateMessageId(icu), message: icu, values: { count: 3 } })).toBe("3 celdas");
    expect(i18n._({ id: generateMessageId(icu), message: icu, values: { count: 1 } })).toBe("1 celda");
    // Not in the file: English
    expect(i18n._({ id: generateMessageId("Show original"), message: "Show original" })).toBe(
      "Show original",
    );
  });

  it("skips plurals it can't map to the language's plural rules", () => {
    const messages = compilePreview({ ...catalog, plural_forms: "nplurals=3; plural=(n%10==1 ? 0 : 1);" });
    expect(messages).not.toHaveProperty(generateMessageId(icu));
  });
});
