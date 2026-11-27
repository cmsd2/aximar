import { defineConfig } from "@lingui/cli";
import { formatter } from "@lingui/format-po-gettext";
import { withoutFuzzy } from "./src/lib/po-format";


// The window's catalogs; see docs/i18n.md.  `scripts/i18n-extract` sets
// this to write the window's part of the template elsewhere, then joins it
// with the Rust crates' part into locales/messages.pot.
const templateDir = process.env.AXIMAR_I18N_TEMPLATE_DIR;

export default defineConfig({
  sourceLocale: "en",
  // `pseudo` is for development only: see src/lib/i18n.ts
  locales: ["en", "es", "pseudo"],
  pseudoLocale: "pseudo",
  fallbackLocales: { default: "en" },
  catalogs: [
    {
      path: templateDir ? `${templateDir}/{locale}` : "<rootDir>/locales/{locale}",
      include: ["<rootDir>/src"],
      exclude: ["**/*.test.ts", "**/*.test.tsx", "**/*.stories.tsx", "<rootDir>/src/stories/**"],
    },
  ],
  // No line numbers, so moving code doesn't change the template
  format: withoutFuzzy(formatter({ lineNumbers: false })),
});
