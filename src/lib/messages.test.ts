import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";
import { i18n } from "./i18n";

/** The template's Rust messages with an ID: context, English and placeholders */
function rustMessages(): { context: string; template: string; params: string[] }[] {
  const pot = readFileSync(new URL("../../locales/messages.pot", import.meta.url), "utf8");
  const unquote = (s: string) => JSON.parse(s) as string;
  const out = [];
  for (const entry of pot.split("\n\n")) {
    if (!entry.includes("#, rust-format") && !/^msgctxt "(error|question|note)\./m.test(entry)) continue;
    const context = entry.match(/^msgctxt (".*")$/m);
    const msgid = entry.match(/^msgid (".*")$/m);
    if (!context || !msgid) continue;
    const template = unquote(msgid[1]);
    const params = [...template.matchAll(/\{(\w+)\}/g)].map((m) => m[1]);
    out.push({ context: unquote(context[1]), template, params });
  }
  return out;
}

describe("the window's English catalog", () => {
  it("renders every Rust message by its context and template", () => {
    const messages = rustMessages();
    expect(messages.length).toBeGreaterThan(20);
    for (const { context, template, params } of messages) {
      const id = generateMessageId(template, context);
      expect(i18n.messages, `${context}: ${template}`).toHaveProperty(id);
      const values = Object.fromEntries(params.map((p) => [p, `<${p}>`]));
      const expected = template.replace(/\{(\w+)\}/g, (_, p) => `<${p}>`);
      expect(i18n._({ id, message: template, values })).toBe(expected);
    }
  });
});
