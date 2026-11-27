# Licences

Aximar ships code and fonts from open-source packages, and their licences
ask that copies carry each package's copyright notice and licence text.
The built app holds them in `licenses/`.

| File | What it covers | Where it comes from |
| --- | --- | --- |
| `JetBrainsMono-OFL.txt` | The monospace font | `public/licenses/`, copied from `@fontsource/jetbrains-mono` |
| `ComputerModern-OFL.txt` | The Computer Modern Markdown font | `public/licenses/`, copied from `computer-modern` |
| `third-party-npm.html` | Every npm package in the window's bundle | Written by `npm run build` |
| `third-party-rust.html` | Every Rust crate the app and the tools link | `public/licenses/`, written by `npm run licences:rust` |

The tools release archives carry `third-party-rust.html` as
`THIRD-PARTY-RUST.html`, beside `maxima-lsp`, `maxima-dap` and
`aximar-mcp`.

Aximar itself is GPL-3.0-or-later (`LICENSE.txt`), which every licence on
the accepted list below is compatible with.

## The accepted licences

`scripts/accepted-licences.ts` lists the licences Aximar accepts, and
`about.toml` holds the same list for the Rust crates; a test fails if the
two differ. In order of preference: MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception,
BSD-3-Clause, BSD-2-Clause, ISC, Zlib, 0BSD, MIT-0, Unicode-3.0, CC0-1.0,
BSL-1.0, Unlicense, OFL-1.1 and MPL-2.0.

A package that offers a choice, such as `MIT OR Apache-2.0`, is used under
the first one in that order, and its notice says so. A package that needs
two, such as `MIT AND BSD-3-Clause`, needs both on the list.

MPL-2.0 is on the list because its copyleft covers only the MPL files
themselves: using them unmodified, with a pointer to their source, meets
it. Everything else on the list asks only for the notice.

## The npm notices

`rollup-plugin-license`, in `vite.config.ts`, sees every module Rollup puts
in the window's bundle, and writes `dist/licenses/third-party-npm.html` from
`scripts/npm-notices.ts`, in the same shape as the Rust page: a contents
list by licence, then each licence text once, with the packages Aximar uses
under it, each linking its source. A package's text is its licence and
notice files. A
package with no licence file gets its copyright line from `package.json`
and, for MIT, the licence's standard text. Packages that only run at build
time, such as Lingui's command line and Babel, never reach the bundle and
aren't listed.

The plugin only sees JavaScript. A package imported only for its CSS and
fonts, such as `@fontsource/jetbrains-mono` and `computer-modern`, needs
its licence copied into `public/licenses/` by hand, and a line in
`public/licenses/README.txt`.

The plugin doesn't run in `vite dev` or in Storybook's build, so a
development build has no npm notices; the Licences window says so, rather
than showing the app's page that the development server sends for a
missing file.

## The Licences window

Settings, General, Licences opens `licenses/index.html`, a viewer whose
tabs (Overview, npm packages, Rust crates, Fonts) stay along the top while
`licenses/viewer.js` loads each notice below them, so there is always a
way back. Links to a package's source open in the system browser.

### When the build fails on a licence

`npm run build` stops with the package's name and licence when a package in
the bundle has a licence outside the list, or none. Then:

1. Check the package's own repository: a missing or odd `license` field in
   `package.json` is often a packaging slip, and a newer version may fix it.
2. Look for an alternative package under an accepted licence.
3. If the package is the right one and its licence is acceptable, add the
   licence to `ACCEPTED_LICENCES`, with a sentence here on why. That's a
   decision about what Aximar may ship, so it goes in its own commit.

## The Rust notices

`scripts/rust-notices.mjs` runs [cargo-about](https://github.com/EmbarkStudios/cargo-about)
once for each shipped binary: the desktop app (`src-tauri`), `maxima-lsp`,
`maxima-dap` and `aximar-mcp`. It covers every platform Aximar builds for,
so a crate only Windows or Linux uses is listed too, and leaves out build
and test dependencies, and development tools such as `catalog-gen`. It
merges the four results, leaves out Aximar's own crates, and writes
`public/licenses/third-party-rust.html`: each licence text once, with the
crates that use it, and a link to each crate's source on docs.rs. That
link meets MPL-2.0's requirement to say where its files' source is.

The file is checked in, so a development build ships it too. When
`Cargo.lock` changes, regenerate it:

```sh
cargo install cargo-about --version 0.9.3 --locked --features cli
npm run licences:rust
```

CI's `licences` job runs `npm run licences:rust -- --check`, which fails
when the file doesn't match what the crates would produce. The run takes
about half a minute and fetches licence files the crates don't package
from their repositories.

`about.toml` names Aximar's own crates, each with its own licence, so
cargo-about accepts them; the script then leaves them out, since they're
the app, not third-party code. A new crate in this repository that a
shipped binary links needs a `license` field and an entry there.

### When generating fails on a licence

cargo-about prints the crate and its licence. The steps are the same as
for npm packages above: check the crate, look for an alternative, or add
the licence to both lists in its own commit, with a sentence here on why.
