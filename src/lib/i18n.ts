// The window's translations.  English is the source language, and the only
// one so far; see docs/i18n.md.
import { i18n } from "@lingui/core";
import { messages as en } from "../../locales/en.po";

i18n.load("en", en);
i18n.activate("en");

export { i18n };

/**
 * In a development build, `VITE_AXIMAR_LOCALE=pseudo npm run tauri dev`
 * shows the window in Lingui's pseudo-locale, accented and bracketed, so
 * any text still plain English isn't in the catalog yet.  Release builds
 * leave this out.
 */
export async function activateDevelopmentLocale(): Promise<void> {
  if (import.meta.env.DEV && import.meta.env.VITE_AXIMAR_LOCALE === "pseudo") {
    const { messages } = await import("../../locales/pseudo.po");
    i18n.load("pseudo", messages);
    i18n.activate("pseudo");
  }
}
