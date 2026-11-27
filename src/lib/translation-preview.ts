// Translation preview, for translators: a .po read from disk by the
// backend (translation_preview.rs), compiled here and shown in place of
// the built-in catalog.  Only in pre-release builds.
import type { Messages } from "@lingui/core";
import { invoke } from "@tauri-apps/api/core";
import { compileMessage } from "@lingui/message-utils/compileMessage";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";
import { englishMessages, i18n, setLanguageOverride } from "./i18n";
import { parseTemplate, setShowSources, showingSources, type Source } from "./show-sources";

/**
 * Whether this build offers the translator tools: development builds, and
 * builds made with VITE_AXIMAR_TRANSLATOR_TOOLS=1 (see docs/i18n.md)
 */
export const TRANSLATOR_TOOLS =
  import.meta.env.DEV || import.meta.env.VITE_AXIMAR_TRANSLATOR_TOOLS === "1";

/** One translated entry, as the backend reads it */
export interface PreviewEntry {
  context: string | null;
  msgid: string;
  msgid_plural: string | null;
  msgstr: string;
  msgstr_plural: string[];
  comments: string[];
}

export interface PreviewCatalog {
  path: string;
  language: string;
  plural_forms: string | null;
  entries: PreviewEntry[];
}

/** The plural rules a preview can show: one form for 1, another for the rest */
function hasOneOther(pluralForms: string | null): boolean {
  if (!pluralForms) return false;
  const rule = pluralForms.replace(/\s/g, "");
  return rule.includes("nplurals=2") && (rule.includes("plural=(n!=1)") || rule.includes("plural=n!=1") || rule.includes("plural=(n>1)"));
}

/**
 * A catalog's translations, compiled for Lingui under the IDs the window
 * looks them up by.  A plural entry carries its ICU message in a
 * `js-lingui:icu=` comment; previews show plurals for languages with one
 * form for 1 and one for the rest, and English for other plural rules.
 */
export function compilePreview(catalog: PreviewCatalog): Messages {
  const messages: Messages = {};
  const oneOther = hasOneOther(catalog.plural_forms);
  for (const entry of catalog.entries) {
    const context = entry.context ?? undefined;
    if (entry.msgid_plural) {
      const lingui = entry.comments.find((c) => c.startsWith("js-lingui:"));
      if (!lingui || !oneOther || entry.msgstr_plural.length < 2) continue;
      const params = new URLSearchParams(lingui.slice("js-lingui:".length));
      const icu = params.get("icu");
      const on = params.get("pluralize_on");
      if (!icu || !on) continue;
      const [one, other] = entry.msgstr_plural;
      const translation = `{${on}, plural, one {${one}} other {${other}}}`;
      messages[generateMessageId(icu, context)] = compileMessage(translation);
    } else if (entry.msgstr) {
      messages[generateMessageId(entry.msgid, context)] = compileMessage(entry.msgstr);
    }
  }
  return messages;
}

/** Show a preview: its translations over English, in its language */
export async function applyPreview(catalog: PreviewCatalog): Promise<void> {
  const sourcesOn = showingSources(i18n);
  await setLanguageOverride(true);
  i18n.load(catalog.language, { ...englishMessages, ...compilePreview(catalog) });
  i18n.activate(catalog.language);
  if (sourcesOn) setShowSources(i18n, true);
}

let sourcesCache: Map<string, Source> | null = null;

/** The template's entries, for show-sources; read on first use */
export async function loadSources(): Promise<Map<string, Source>> {
  if (!sourcesCache) {
    const { default: pot } = await import("../../locales/messages.pot?raw");
    sourcesCache = parseTemplate(pot);
  }
  return sourcesCache;
}

/** The file being previewed, if any, kept while Settings is closed */
let previewPath: string | null = null;

export function currentPreviewPath(): string | null {
  return previewPath;
}

/** Load a .po from disk and keep showing it as it's saved */
export async function startPreview(path: string): Promise<void> {
  const catalog = await invoke<PreviewCatalog>("load_translation_preview", { path });
  previewPath = catalog.path;
  await applyPreview(catalog);
}

/** Stop previewing and go back to `language`, the setting */
export async function stopPreview(language: string): Promise<void> {
  await invoke("stop_translation_preview");
  previewPath = null;
  await setLanguageOverride(false, language);
}
