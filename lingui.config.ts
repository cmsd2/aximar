import { defineConfig } from "@lingui/cli";
import { formatter } from "@lingui/format-po-gettext";

// The window's catalogs; see docs/i18n.md.  `scripts/i18n-extract` sets
// this to write the window's part of the template elsewhere, then joins it
// with the Rust crates' part into locales/messages.pot.
const templateDir = process.env.AXIMAR_I18N_TEMPLATE_DIR;

export default defineConfig({
  sourceLocale: "en",
  // `pseudo` is for development only: see src/lib/i18n.ts
  locales: ["en", "pseudo"],
  pseudoLocale: "pseudo",
  fallbackLocales: { default: "en" },
  catalogs: [
    {
      path: templateDir ? `${templateDir}/{locale}` : "<rootDir>/locales/{locale}",
      include: ["<rootDir>/src"],
      exclude: ["**/*.test.ts", "**/*.test.tsx"],
    },
  ],
  // No line numbers, so moving code doesn't change the template
  format: formatter({ lineNumbers: false }),
});
