import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { lingui } from "@lingui/vite-plugin";
import license from "rollup-plugin-license";
import { thirdPartyNotices } from "./scripts/npm-notices";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
// Storybook builds with this config too; it ships no notices
// @ts-expect-error process is a nodejs global
const storybook = process.env.STORYBOOK === "true";

// https://vite.dev/config/
export default defineConfig(async () => ({
  // Lingui compiles the .po catalogs on import, and its macros natively
  plugins: [react(), lingui({ macroTransform: true })],

  // The app's build writes the notices of the npm packages in its bundle,
  // and fails on a licence Aximar hasn't accepted.  See docs/licences.md.
  build: {
    rollupOptions: {
      plugins: storybook ? [] : [license({ thirdParty: thirdPartyNotices("dist/licenses/third-party-npm.txt") })],
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
