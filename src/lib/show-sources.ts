// Show sources, for translators in pre-release builds: hold Alt and point
// at any translated text to see its English, context, note and source
// file.
//
// At run time Lingui knows a string only by its hash ID.  While the mode
// is on, every translated string ends with its ID, encoded in invisible
// characters; pointing at text decodes the IDs in it and looks them up in
// the template, which holds the rest.  The mode is a private-use alias of
// the language (`es-x-src`), so that switching it re-renders the window,
// as a language switch does.
import type { I18n } from "@lingui/core";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";

const SUFFIX = "-x-src";
const START = "⁣";
const END = "⁤";
const BITS = ["​", "‌"];

/** An ID as invisible characters */
export function encodeId(id: string): string {
  let out = START;
  for (const char of id) {
    const code = char.codePointAt(0)!;
    for (let bit = 7; bit >= 0; bit--) out += BITS[(code >> bit) & 1];
  }
  return out + END;
}

/** The IDs encoded in some text */
export function decodeIds(text: string): string[] {
  const ids: string[] = [];
  const pattern = new RegExp(`${START}([${BITS.join("")}]+)${END}`, "g");
  for (const match of text.matchAll(pattern)) {
    let id = "";
    const bits = match[1];
    for (let i = 0; i + 8 <= bits.length; i += 8) {
      let code = 0;
      for (const b of bits.slice(i, i + 8)) code = (code << 1) | (b === BITS[1] ? 1 : 0);
      id += String.fromCodePoint(code);
    }
    ids.push(id);
  }
  return ids;
}

/** Whether `i18n` is showing sources */
export function showingSources(i18n: I18n): boolean {
  return i18n.locale.endsWith(SUFFIX);
}

/**
 * Make `i18n` tag each string with its ID while sources are shown.  Called
 * once, before the window renders, in pre-release builds.
 */
export function installSourceTags(i18n: I18n): void {
  const translate = i18n._.bind(i18n);
  const tagged: typeof i18n._ = ((id: unknown, values?: unknown, options?: unknown) => {
    const out = (translate as (...args: unknown[]) => string)(id, values, options);
    if (!showingSources(i18n)) return out;
    const messageId = typeof id === "string" ? id : (id as { id?: string } | undefined)?.id;
    return messageId ? out + encodeId(messageId) : out;
  }) as typeof i18n._;
  i18n._ = tagged;
  i18n.t = tagged;
}

/** Turn showing sources on or off, keeping the language and its catalog */
export function setShowSources(i18n: I18n, on: boolean): void {
  const language = i18n.locale.endsWith(SUFFIX) ? i18n.locale.slice(0, -SUFFIX.length) : i18n.locale;
  if (on) {
    i18n.load(language + SUFFIX, i18n.messages);
    i18n.activate(language + SUFFIX);
  } else {
    i18n.activate(language);
  }
}

/** What the template says about one entry */
export interface Source {
  id: string;
  msgid: string;
  context: string | null;
  notes: string[];
  files: string[];
}

/** The template's entries by ID, as the window looks them up */
export function parseTemplate(pot: string): Map<string, Source> {
  const sources = new Map<string, Source>();
  for (const block of pot.split(/\n\s*\n/)) {
    const lines = block.split("\n");
    const field = (name: string) => {
      const line = lines.find((l) => l.startsWith(`${name} "`));
      return line === undefined ? null : (JSON.parse(line.slice(name.length + 1)) as string);
    };
    const msgid = field("msgid");
    if (!msgid) continue;
    const context = field("msgctxt");
    const notes = lines.filter((l) => l.startsWith("#. ")).map((l) => l.slice(3));
    const files = lines
      .filter((l) => l.startsWith("#: "))
      .flatMap((l) => l.slice(3).split(" "))
      .filter(Boolean);
    // A plural's ID comes from its ICU message, which Lingui keeps in a note
    const lingui = notes.find((n) => n.startsWith("js-lingui:"));
    const icu = lingui ? new URLSearchParams(lingui.slice("js-lingui:".length)).get("icu") : null;
    const id = generateMessageId(icu ?? msgid, context ?? undefined);
    sources.set(id, {
      id,
      msgid,
      context,
      notes: notes.filter((n) => !n.startsWith("js-lingui:")),
      files,
    });
  }
  return sources;
}
