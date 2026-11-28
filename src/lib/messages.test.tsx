import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";
import { setupI18n } from "@lingui/core";
import { compileMessage } from "@lingui/message-utils/compileMessage";
import { renderToStaticMarkup } from "react-dom/server";
import { i18n } from "./i18n";
import { messageContent, messageParts, messageText } from "./messages";
import type { Message } from "../types/maxima";

/** The template's Rust messages with an ID: context, English and placeholders */
function rustMessages(): { context: string; template: string; params: string[] }[] {
  const pot = readFileSync(new URL("../../locales/messages.pot", import.meta.url), "utf8")
    // A Windows checkout has CRLF line endings
    .replace(/\r\n/g, "\n");
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

describe("code in a message", () => {
  const context = "note.test";
  /** An i18n with one translation, of `template` as `translation` */
  function translated(template: string, translation: string) {
    const i18n = setupI18n();
    i18n.load("xx", { [generateMessageId(template, context)]: compileMessage(translation) });
    i18n.activate("xx");
    return i18n;
  }
  const message = (template: string, params: Record<string, string> = {}): Message => ({
    id: context,
    template,
    params,
    text: template.replace(/\{(\w+)\}/g, (_, p: string) => params[p]),
  });

  it("splits a code span out of the prose", () => {
    const i18n = translated("Use `solve` here.", "Usa `solve` aquí.");
    expect(messageParts(i18n, message("Use `solve` here."))).toEqual([
      { code: false, text: "Usa " },
      { code: true, text: "solve" },
      { code: false, text: " aquí." },
    ]);
  });

  it("puts a parameter's value inside its code span", () => {
    const template = "Maxima replaced decimals with exact fractions: `{replacements}`.";
    const i18n = translated(template, "Maxima cambió decimales por fracciones: `{replacements}`.");
    expect(messageParts(i18n, message(template, { replacements: "0.5 → 1/2" }))).toEqual([
      { code: false, text: "Maxima cambió decimales por fracciones: " },
      { code: true, text: "0.5 → 1/2" },
      { code: false, text: "." },
    ]);
  });

  it("keeps a backquote in a value inside its span", () => {
    const template = "Maxima quoted `{expr}` with {how}.";
    const parts = messageParts(setupI18n(), message(template, { expr: "'`f(x)", how: "a `backquote" }));
    expect(parts).toEqual([
      { code: false, text: "Maxima quoted " },
      { code: true, text: "'`f(x)" },
      { code: false, text: " with a `backquote." },
    ]);
  });

  it("reads a backtick without a partner as prose", () => {
    const template = "Use `solve` here.";
    const i18n = translated(template, "Usa `solve` aquí, o `algsys.");
    expect(messageParts(i18n, message(template))).toEqual([
      { code: false, text: "Usa " },
      { code: true, text: "solve" },
      { code: false, text: " aquí, o `algsys." },
    ]);
  });

  it("leaves a message without backticks as prose", () => {
    const template = "Maxima needs to know the sign of {expr}";
    expect(messageParts(setupI18n(), message(template, { expr: "a" }))).toEqual([
      { code: false, text: "Maxima needs to know the sign of a" },
    ]);
  });

  it("falls back to English for a message the catalog lacks", () => {
    const template = "Use `{f}` instead.";
    expect(messageText(setupI18n(), message(template, { f: "ratsimp" }))).toBe("Use `ratsimp` instead.");
  });

  it("renders code spans as <code>", () => {
    const html = renderToStaticMarkup(<>{messageContent(setupI18n(), message("Use `{f}`.", { f: "a<b" }))}</>);
    expect(html).toBe('Use <code class="message-code">a&lt;b</code>.');
  });
});
