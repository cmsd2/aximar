#!/usr/bin/env node
// Adds `#. screen: <story>` comments to a template, from
// locales/screens.json, which scripts/i18n-gallery.mjs writes: the
// Storybook stories each string appears in.  Run by scripts/i18n-extract.
//
//   node scripts/i18n-screens.mjs path/to/messages.pot
import { readFileSync, writeFileSync } from "node:fs";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";

const [potPath] = process.argv.slice(2);
const screens = JSON.parse(readFileSync(new URL("../locales/screens.json", import.meta.url), "utf8"));
const pot = readFileSync(potPath, "utf8");

const out = pot.split(/\n\n/).map((block) => {
  const lines = block.split("\n");
  const field = (name) => {
    const line = lines.find((l) => l.startsWith(`${name} "`));
    return line === undefined ? undefined : JSON.parse(line.slice(name.length + 1));
  };
  const msgid = field("msgid");
  if (!msgid) return block;
  // A plural's ID comes from its ICU message, which Lingui keeps in a note
  const lingui = lines.find((l) => l.startsWith("#. js-lingui:"));
  const icu = lingui ? new URLSearchParams(lingui.slice("#. js-lingui:".length)).get("icu") : null;
  const stories = screens[generateMessageId(icu ?? msgid, field("msgctxt"))];
  if (!stories) return block;
  // After the other notes, before the references
  const at = lines.findIndex((l) => !l.startsWith("#."));
  lines.splice(at, 0, ...stories.map((s) => `#. screen: ${s}`));
  return lines.join("\n");
});
writeFileSync(potPath, out.join("\n\n"));
