// Aximar's explanations of Maxima's output, in the user's language.  The
// core sends each in English with its template and parameters (see
// aximar-core's message.rs); the window looks the template up in the
// catalog under the message's ID.
import type { I18n } from "@lingui/core";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";
import type { ErrorInfo } from "../types/maxima";

/** An explanation's translatable fields */
export type ErrorField = "title" | "explanation" | "suggestion";

/**
 * A field of an explanation in the active language, or its English text
 * when the catalog doesn't have it
 */
export function errorText(i18n: I18n, info: ErrorInfo, field: ErrorField): string {
  const text = info[field] ?? "";
  const source = info.messages?.[field];
  if (!source) return text;
  const id = generateMessageId(source.template, source.id);
  if (!(id in i18n.messages)) return text;
  // The core's message, already in the catalog: not one to extract here
  /* lingui-extract-ignore */
  return i18n._({ id, message: source.template, values: info.params ?? {} });
}
