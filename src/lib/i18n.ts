// The window's translations.  English is the source language, built in;
// other languages load when chosen.  See docs/i18n.md.
import { i18n, type Messages } from "@lingui/core";
import { messages as en } from "../../locales/en.po";

i18n.load("en", en);
i18n.activate("en");

export { i18n };

/** The English catalog, which other languages fall back to */
export const englishMessages: Messages = en;

/** Each language's catalog, compiled by the Lingui Vite plugin on import */
const catalogs = import.meta.glob<{ messages: Messages }>([
  "../../locales/*.po",
  "!../../locales/en.po",
  "!../../locales/pseudo.po",
]);

/**
 * True while the pseudo-locale or a translation preview is on: the
 * language setting doesn't override either
 */
let override = false;

/** Turn the override on or off; off goes back to `language` */
export async function setLanguageOverride(on: boolean, language?: string): Promise<void> {
  override = on;
  if (!on && language) await activateLanguage(language);
}

/**
 * Show the window in `language`, such as `es`: at start, and when the
 * setting changes.  English for a language without a catalog.  The
 * I18nProvider re-renders the window.
 */
export async function activateLanguage(language: string): Promise<void> {
  if (override) return;
  const load = catalogs[`../../locales/${language}.po`];
  if (language === "en" || !load) {
    i18n.activate("en");
    return;
  }
  const { messages } = await load();
  i18n.load(language, messages);
  i18n.activate(language);
}

/** A language's own name, such as "Español", for the language setting */
export function languageName(language: string): string {
  try {
    const name = new Intl.DisplayNames([language], { type: "language" }).of(language) ?? language;
    return name.charAt(0).toLocaleUpperCase(language) + name.slice(1);
  } catch {
    return language;
  }
}

/**
 * In a development build, `VITE_AXIMAR_LOCALE=pseudo npm run tauri dev`
 * shows the window in Lingui's pseudo-locale, accented and bracketed, so
 * any text still plain English isn't in the catalog yet.  Release builds
 * leave this out.
 */
export async function activateDevelopmentLocale(): Promise<void> {
  if (import.meta.env.DEV && import.meta.env.VITE_AXIMAR_LOCALE === "pseudo") {
    override = true;
    const { messages } = await import("../../locales/pseudo.po");
    i18n.load("pseudo", messages);
    i18n.activate("pseudo");
  }
}
