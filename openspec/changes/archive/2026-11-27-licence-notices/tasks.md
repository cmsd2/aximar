## 1. npm notices in the build (first PR)

- [x] 1.1 Add `rollup-plugin-license` 3.7.2, pinned, as a dev dependency, and configure it in `vite.config.ts` for `vite build` only, writing `dist/licenses/third-party-npm.html` and failing on a licence outside the accepted list; verify `npm run build` writes the file, that it lists `react` and `katex` and not `caniuse-lite` or `@lingui/cli`, and that Storybook and `vite dev` still start
- [x] 1.2 Add a vitest test `npm_licences_are_accepted` that runs the plugin's accepted-list check over a fixture of package licences (MIT; "MIT OR Apache-2.0"; "(MIT AND Zlib)"; GPL-3.0-only; a package with none); verify the GPL and unlicensed cases fail and the rest pass
- [x] 1.3 Document the npm side in `docs/licences.md`: what the file holds, the accepted list, and what to do when a new dependency fails it

## 2. Rust notices, checked in and checked (one PR)

- [x] 2.1 Add `about.toml` (accepted list in order, the five build targets, Aximar's own crates by name) and `scripts/rust-notices.mjs` (one cargo-about run per shipped binary, merged, Aximar's crates left out, licences grouped with each text once, every crate linking its source), and `npm run licences:rust`; give `maxima-lsp` and `maxima-mac-parser` the repository's licence; generate `public/licenses/third-party-rust.html`; verify the file lists `tauri`, `aximar-core`'s dependencies, a Windows-only crate such as `windows-sys`, and links `cssparser`'s source; test the merge and render with `node --test`, and that `about.toml` and `scripts/accepted-licences.ts` list the same licences
- [x] 2.2 Add the CI job `licences` that installs `cargo-about` 0.9.3 with `taiki-e/install-action` and runs `npm run licences:rust -- --check`, which fails on a stale file and names the command; verify it passes on this branch, and that generating fails when a licence in use leaves the accepted list
- [x] 2.3 Copy the file into the tools archives as `THIRD-PARTY-RUST.html` in `release-tools.yml`, for the tar and the zip; verify by running the Unix packaging step on placeholder binaries and listing the archive, and actionlint on the workflow
- [x] 2.4 Add the Rust side to `docs/licences.md`

## 3. Reaching the notices from the app (one PR)

- [x] 3.1 Write `public/licenses/index.html`, linking the two font licences, `third-party-npm.html` and `third-party-rust.html`; verify every link resolves in `dist/` after `npm run build`, with a vitest test that reads the page and checks each `href` names a file the build writes or `public/licenses/` holds
- [x] 3.2 Add the Tauri command `open_licences`, which opens or focuses a `licences` window at `licenses/index.html`; verify with a Rust test of the window's page and size; by hand that it opens, opens once, and works with the network off is left for the in-app check
- [x] 3.3 Add the Licences row to Settings, General, with a hint and a Show licences button that calls `open_licences`; add the strings to the catalog with translator notes; verify with a vitest test that the button calls the command, the settings-sections tests with the new row, and the General story in the gallery
- [x] 3.4 Set `AboutMetadata.credits` to the translated "Open-source licences: Settings, General, Licences" on macOS and Linux; also set the panel's licence field to GPL-3.0-or-later; verify with a `menu.rs` test of the label, and by hand in the About panel (left for the in-app check)
- [x] 3.5 Add a line on the licences to `README.md`, and verify `openspec validate licence-notices --strict` passes
