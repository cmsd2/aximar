#!/usr/bin/env node
// Warns about translations whose backticks don't match their English.
// Backticks mark code in the core's messages ("use `solve()`"), and the
// window shows each pair as code; a translation that loses one shows the
// rest of the message as prose.  Run by scripts/i18n-extract.
//
//   node scripts/i18n-backticks.mjs locales/es.po ...
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const count = (s) => s.split("`").length - 1;

/**
 * The translated entries of a catalog (written with --no-wrap) whose
 * backtick count differs from their English: context, English and
 * translation.  Untranslated, fuzzy and obsolete entries don't count.
 */
export function mismatches(po) {
  const out = [];
  for (const block of po.split(/\n\n+/)) {
    const lines = block.split("\n");
    if (lines.some((l) => l.startsWith("#,") && l.includes("fuzzy"))) continue;
    const fields = {};
    for (const line of lines) {
      const m = line.match(/^(msgctxt|msgid|msgid_plural|msgstr(?:\[\d+\])?) (".*")$/);
      if (m) fields[m[1]] = JSON.parse(m[2]);
    }
    if (!fields.msgid) continue;
    const english = fields.msgid_plural ?? fields.msgid;
    for (const [name, translation] of Object.entries(fields)) {
      if (!name.startsWith("msgstr") || translation === "") continue;
      if (count(translation) !== count(english)) {
        out.push({ context: fields.msgctxt ?? "", english, translation });
      }
    }
  }
  return out;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  for (const path of process.argv.slice(2)) {
    for (const { context, english, translation } of mismatches(readFileSync(path, "utf8"))) {
      const where = context ? ` (${context})` : "";
      console.error(`warning: ${path}${where}: backticks: ${count(translation)} in the translation, ${count(english)} in the English`);
      console.error(`  English:     ${english}`);
      console.error(`  translation: ${translation}`);
    }
  }
}
