import type { StorybookConfig } from "@storybook/react-vite";

// The translators' gallery: the window's states in every language.
// See docs/i18n.md, "Context for translators".
const config: StorybookConfig = {
  stories: ["../src/**/*.stories.tsx"],
  framework: { name: "@storybook/react-vite", options: {} },
  // The window's Vite config (Lingui's plugin included) applies as it is
  core: { disableTelemetry: true },
};

export default config;
