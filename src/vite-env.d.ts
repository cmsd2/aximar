/// <reference types="vite/client" />

// A catalog, compiled on import by @lingui/vite-plugin
declare module "*.po" {
  import type { Messages } from "@lingui/core";
  export const messages: Messages;
}

interface ImportMetaEnv {
  /** `pseudo` for the pseudo-locale, in development builds only */
  readonly VITE_AXIMAR_LOCALE?: string;
  /** `1` for a translators' pre-release build, with the translation preview */
  readonly VITE_AXIMAR_TRANSLATOR_TOOLS?: string;
}
