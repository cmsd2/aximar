import type { I18n, MessageDescriptor } from "@lingui/core";

/** An option's label: a message to translate, or a product name kept as it is */
export type Label = MessageDescriptor | string;

export function labelText(i18n: I18n, label: Label): string {
  return typeof label === "string" ? label : i18n._(label);
}
