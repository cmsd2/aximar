## Context

The desktop app is a Tauri 2 shell (`src-tauri`) around a Vite-built window (`src/`, built into `dist/`). Its notices today are two font licences in `public/licenses/`, which Vite copies into `dist/licenses/`. The tools release (`.github/workflows/release-tools.yml`) builds `maxima-lsp`, `maxima-dap` and `aximar-mcp` and packs the bare binaries into `aximar-tools-<target>.tar.gz` or `.zip`.

A survey of the dependency trees on 27 November 2026:

- **npm**: 195 production packages, of which 179 are MIT, 7 ISC, and the rest Apache-2.0, BSD-2-Clause, dual MIT/Apache, OFL-1.1 (the font) and CC-BY-4.0. The CC-BY-4.0 one is `caniuse-lite`, reached through `@lingui/cli`'s Babel, which only runs at build time.
- **Rust**: 558 crates in the app's normal dependency graph across platforms. Almost all are MIT, Apache-2.0 or a choice of the two. The rest are Unicode-3.0 (22, the ICU data crates), Zlib, BSD, ISC, CC0, BSL-1.0 and Unlicense alternatives, two offering LGPL-2.1-or-later as one choice among MIT and Apache, and 8 releases of 5 crates under MPL-2.0 (`cssparser`, `cssparser-macros`, `selectors`, `dtoa-short`, `option-ext`, `typed-generational-arena`).

`cargo-deny` 0.20.2 is installed on the development machine, and `cargo-about` isn't.

## Goals / Non-Goals

**Goals:**

- A notice for every package the shipped artefacts contain, generated rather than written by hand.
- A build that stops on a licence nobody has accepted.
- Notices a user can find from the app, offline.

**Non-Goals:**

- The Docker image, the VS Code extension and the notebook-format npm package (see the proposal).
- Deciding licence policy beyond the accepted list. Anything outside it stops the build and goes to the user.

## Decisions

### 1. npm: `rollup-plugin-license` in the Vite build

`rollup-plugin-license` (3.7.2, pinned, a dev dependency) runs in `vite.config.ts` and sees the modules Rollup actually bundles, so build tooling stays out without a hand-kept exclude list. Its `thirdParty.output` writes `dist/licenses/third-party-npm.txt`. Its `thirdParty.allow.test` is the accepted list as an SPDX expression check, with `failOnUnlicensed` and `failOnViolation` set, so a violation fails `npm run build`. It runs only in `vite build`, not in `vite dev` or Storybook.

Alternative: `license-checker` over `npm ls --omit=dev`. It reports packages that never reach the bundle, such as `caniuse-lite`, so the list would need excludes that go stale. Rejected.

### 2. Rust: `cargo-about`, checked in, checked in CI

`cargo-about` (0.9.3) generates a notices file from a Handlebars template, reads each crate's licence files from the registry source, and resolves a choice of licences against an accepted list in order. That suits the job better than `cargo-deny`, which can check licences but not write notices.

- `about.toml` holds the accepted list, in this order of preference: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-3-Clause, BSD-2-Clause, ISC, Zlib, 0BSD, MIT-0, Unicode-3.0, CC0-1.0, BSL-1.0, Unlicense, MPL-2.0. MIT first means a crate offering "MIT OR Apache-2.0 OR LGPL-2.1-or-later" is used under MIT.
- It lists `targets` for every platform Aximar builds (macOS arm64 and x64, Linux x64 and arm64, Windows x64), so the file covers Windows-only and Linux-only crates as well.
- `about.hbs` groups crates by licence: each licence text once, then the crates that use it, with versions. A crate under MPL-2.0 also gets a link to its source on crates.io (`https://crates.io/crates/<name>/<version>/source`), which meets MPL-2.0's requirement to tell recipients where the source is.
- `npm run licences:rust` runs `cargo about generate --manifest-path Cargo.toml --workspace --fail about.hbs -o public/licenses/third-party-rust.html`. `--workspace` covers the four shipped binaries, and test-only and build-only crates stay out, since `cargo-about` follows normal dependencies only.

The file is checked in rather than generated in each build, because the Tauri build doesn't run Cargo before Vite, and a developer's build should be as complete as a release. A CI job, `licences`, installs `cargo-about` with `taiki-e/install-action` and runs the script, and fails if `git diff --exit-code public/licenses/third-party-rust.html` shows a change. It runs on every pull request; the file only changes when `Cargo.lock` does.

Alternative: generate in `build.yml` only. Development builds would show an empty Rust list, and nothing would catch a new licence until release. Rejected.

### 3. One index, one window

`public/licenses/index.html` is a static page, outside the window's React app and stylesheets. It sets `color-scheme: light dark` and the system font, and writes no colours of its own, so the webview's own light and dark colours apply and the design library's rule holds. It links:

- `JetBrainsMono-OFL.txt` and `ComputerModern-OFL.txt`, with the README's lines;
- `third-party-npm.txt`;
- `third-party-rust.html`.

A Tauri command, `open_licences`, opens a `WebviewWindow` labelled `licences` at `licenses/index.html`, sized 720 by 640, or focuses it if it is already open. The window loads only bundled files, so it needs no capability beyond the defaults.

The General section of Settings gains a row, Licences, with the hint "The open-source software and fonts Aximar uses, and their licences" and a Show licences button. The row is a link, not a setting, so the settings-window spec's sections are unchanged.

On macOS and Linux, `AboutMetadata.credits` says "Open-source licences: Settings, General, Licences" in the user's language, through `msg!`.

### 4. The tools archives

`release-tools.yml` copies `public/licenses/third-party-rust.html` into each archive as `THIRD-PARTY-RUST.html`, beside the binaries. It covers all four binaries, which over-lists for the tools; listing a crate a binary doesn't link costs nothing, and one file is simpler to keep current than four.

### What changes for MCP clients and saved notebooks

Nothing.

## Risks / Trade-offs

- **[A crate with no licence file in its package]** → `cargo-about` falls back to the SPDX text for the declared licence and records the crate's authors as the copyright line. Its `--fail` flag fails on a crate with no licence at all, which goes to the user.
- **[The Rust file is large]** → About 560 crates, but each licence text appears once, grouped, so the file is a few hundred KB of HTML. It loads only when opened.
- **[A check that fails on every dependency bump]** → Only when `Cargo.lock` changes, and the fix is one command, which the failure names.
- **[The accepted list is a policy choice]** → It holds only permissive licences and MPL-2.0, whose file-level copyleft is met by unmodified use and a source link. The proposal names this, so the user can change it before the first build fails.
