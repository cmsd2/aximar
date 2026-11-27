## Why

Aximar ships code from about 750 open-source packages and no notice for any of them. MIT, Apache-2.0, BSD, ISC, Zlib and Unicode-3.0 all require their copyright notice and licence text in copies of the software, and MPL-2.0 also requires telling recipients where to get the source. The desktop app bundles about 200 npm packages and links about 560 Rust crates. The tools release archives (`maxima-lsp`, `maxima-dap`, `aximar-mcp`) carry their own share of the crates, also without notices. The design-library change started `public/licenses/` for the two bundled fonts. The rest needs the same, generated so it stays current as dependencies change.

## What Changes

- The Vite build writes `dist/licenses/third-party-npm.html`: every npm package in the window's bundle, with its version, licence and licence text, from `rollup-plugin-license`. Build tooling that never reaches the bundle (such as `caniuse-lite`, which comes in through `@lingui/cli`) stays out.
- `cargo-about` generates `public/licenses/third-party-rust.html` for the Rust crates the shipped binaries link: the desktop app and the three tools. The file is checked in, so every build, a developer's included, ships it, and a CI job regenerates it and fails when it is stale.
- Both generators fail the build on a licence outside an accepted list: MIT, MIT-0, Apache-2.0 (with or without the LLVM exception), BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, Unicode-3.0, CC0-1.0, BSL-1.0, Unlicense, OFL-1.1 and MPL-2.0. A package under anything else, such as GPL, LGPL alone, or CC-BY-SA, stops the build until someone decides.
- MPL-2.0 crates (today `cssparser`, `selectors`, `dtoa-short`, `option-ext` and `typed-generational-arena`, all unmodified) are listed with a link to their source on crates.io, which MPL-2.0 asks for.
- `public/licenses/index.html` links the fonts, the npm notices and the Rust notices. The app opens it in its own window from a new **Licences** row in Settings, General section, and the About panel on macOS and Linux says where to find it.
- The tools release archives include `THIRD-PARTY-RUST.html` beside the binaries.

## Capabilities

### New Capabilities

- `licence-notices`: which notices the app and the tools ship, how they're generated and kept current, which licences are accepted, and how a user reaches them.

### Modified Capabilities

None. `settings-window` gains a row, but its requirements don't change: every setting still appears in one section, and the new row is a link, not a setting.

## Impact

- **Frontend**: `vite.config.ts` (the npm generator), `public/licenses/index.html`, a `LicencesRow` in the General section, and its strings in the catalog.
- **Tauri**: a command that opens the notices window, `src-tauri/src/menu.rs` (the About panel's credits line), and `src-tauri/capabilities/` if the new window needs a capability.
- **Tools and CI**: `about.toml` and `about.hbs` (cargo-about's accepted list and template), `npm run licences:rust`, a CI job checking the Rust file is current, and `release-tools.yml` copying the file into the archives.
- **Dependencies**: `rollup-plugin-license` 3.7.2 (dev, pinned); `cargo-about` 0.9.3, installed in CI and by developers who regenerate the file.
- **Core, MCP**: none.
- **Docs**: `docs/licences.md`, and a line in `README.md`.

## Non-goals

- The `aximar/maxima` Docker image, which ships Maxima under the GPL; its notices belong to the image's own build.
- The VS Code extension, which lives in its own repository and ships its own `node_modules`.
- The `@aximar/notebook-format` npm package, which npm users install with its dependencies' own licence files.
- A legal review. The accepted list follows common practice for permissive licences; the change makes the inventory visible so the user can review it.
