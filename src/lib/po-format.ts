// The catalog format the window's catalogs use; see lingui.config.ts.
import type { CatalogFormatter } from "@lingui/conf";

/**
 * The po-gettext format, but a fuzzy entry has no translation: Lingui would
 * otherwise show msgmerge's guesses before a translator has reviewed them.
 * The menus skip fuzzy entries too (src-tauri/src/i18n.rs).
 */
export function withoutFuzzy(base: CatalogFormatter): CatalogFormatter {
  return {
    ...base,
    async parse(content, ctx) {
      const catalog = await base.parse(content, ctx);
      for (const message of Object.values(catalog ?? {})) {
        const flags = (message.extra as { flags?: string[] } | undefined)?.flags ?? [];
        if (flags.includes("fuzzy")) message.translation = "";
      }
      return catalog;
    },
  };
}
