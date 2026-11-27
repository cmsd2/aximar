// Aximar's explanations of Maxima's output, in the user's language.  The
// core sends each in English with its template and parameters (see
// aximar-core's message.rs); the window looks the template up in the
// catalog under the message's ID.
//
// Templates mark code with backticks: "Maxima replaced decimals with exact
// fractions: `{replacements}`."  The window shows those spans in the code
// font; the plain text keeps the backticks.
import { createElement, Fragment, type ReactNode } from "react";
import type { I18n } from "@lingui/core";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";
import type { ErrorInfo, Message } from "../types/maxima";

/** An explanation's translatable fields */
export type ErrorField = "title" | "explanation" | "suggestion";

/** A run of a message's text, in the prose font or the code font */
export interface MessagePart {
  code: boolean;
  text: string;
}

/** Where a parameter's value goes: its index between private-use characters */
const sentinel = (i: number) => `${i}`;
const SENTINEL = /(\d+)/g;

/** `template` with each placeholder filled from `values`, as the core fills it */
function fill(template: string, values: Record<string, string>): string {
  return template.replace(/\{(\w+)\}/g, (whole, name: string) => values[name] ?? whole);
}

/**
 * A message in the active language with a sentinel for each parameter, and
 * the values the sentinels stand for
 */
function translateMarked(
  i18n: I18n,
  context: string,
  template: string,
  params: Record<string, string>,
): { marked: string; values: string[] } {
  const names = Object.keys(params);
  const values = names.map((n) => params[n]);
  const sentinels = Object.fromEntries(names.map((n, i) => [n, sentinel(i)]));
  const id = generateMessageId(template, context);
  // A message the catalog doesn't have yet reads as the core's English
  const marked =
    id in i18n.messages
      ? // The core's message, already in the catalog: not one to extract here
        /* lingui-extract-ignore */
        i18n._({ id, message: template, values: sentinels })
      : fill(template, sentinels);
  return { marked, values };
}

/**
 * `marked` split on backticks into prose and code, with each parameter's
 * value put back.  Values go in after the split, so a backquote in one
 * can't start or end a code span.  A backtick without a partner stays as
 * written, and the rest reads as prose.
 */
export function splitCode(marked: string, values: string[]): MessagePart[] {
  const pieces = marked.split("`");
  // An even number of pieces means an odd number of backticks: the last
  // one has no partner
  const unbalanced = pieces.length % 2 === 0;
  const parts: MessagePart[] = [];
  pieces.forEach((piece, i) => {
    let code = i % 2 === 1;
    let text = piece;
    if (unbalanced && i === pieces.length - 1) {
      code = false;
      text = "`" + piece;
    }
    text = text.replace(SENTINEL, (_, n: string) => values[Number(n)] ?? "");
    if (text === "") return;
    const last = parts[parts.length - 1];
    if (last && last.code === code) last.text += text;
    else parts.push({ code, text });
  });
  return parts;
}

/** A message's parts in the active language */
function translateParts(
  i18n: I18n,
  context: string,
  template: string,
  params: Record<string, string>,
): MessagePart[] {
  const { marked, values } = translateMarked(i18n, context, template, params);
  return splitCode(marked, values);
}

/** Parts as React nodes, with code in `<code>` */
export function renderParts(parts: MessagePart[]): ReactNode {
  return createElement(
    Fragment,
    null,
    ...parts.map((p, i) => (p.code ? createElement("code", { key: i, className: "message-code" }, p.text) : p.text)),
  );
}

/** The parts' text, with backticks around the code */
function plain(parts: MessagePart[]): string {
  return parts.map((p) => (p.code ? `\`${p.text}\`` : p.text)).join("");
}

/** A field of an explanation in the active language, as prose and code */
export function errorParts(i18n: I18n, info: ErrorInfo, field: ErrorField): MessagePart[] {
  const text = info[field] ?? "";
  const source = info.messages?.[field];
  // Without a template there are no parameters to keep apart
  if (!source) return splitCode(text, []);
  return translateParts(i18n, source.id, source.template, info.params ?? {});
}

/** A message from aximar-core, such as a note's, in the active language, as prose and code */
export function messageParts(i18n: I18n, message: Message): MessagePart[] {
  return translateParts(i18n, message.id, message.template, message.params);
}

/** A field of an explanation in the active language, with code in the code font */
export function errorContent(i18n: I18n, info: ErrorInfo, field: ErrorField): ReactNode {
  return renderParts(errorParts(i18n, info, field));
}

/** A message from aximar-core in the active language, with code in the code font */
export function messageContent(i18n: I18n, message: Message): ReactNode {
  return renderParts(messageParts(i18n, message));
}

/** A field of an explanation in the active language, as plain text with backticks */
export function errorText(i18n: I18n, info: ErrorInfo, field: ErrorField): string {
  return plain(errorParts(i18n, info, field));
}

/** A message from aximar-core in the active language, as plain text with backticks */
export function messageText(i18n: I18n, message: Message): string {
  return plain(messageParts(i18n, message));
}
