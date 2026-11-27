# licence-notices Specification

## Purpose

Aximar ships the copyright notices and licence texts of the open-source code and fonts it bundles, keeps them current as dependencies change, and refuses a dependency under a licence nobody has accepted.

## Requirements

### Requirement: The app ships a notice for every bundled npm package

The build SHALL write `licenses/third-party-npm.html` into the built app, listing every npm package whose code is in the window's bundle with its name, version, licence and a link to its source, under its licence text, or its copyright line where the package has no licence file. Packages sharing a licence text SHALL share one copy of it. It SHALL NOT list packages used only to build.

#### Scenario: A bundled package is listed

- **WHEN** `npm run build` finishes
- **THEN** `dist/licenses/third-party-npm.html` lists `react` and `katex` under MIT licence texts, each with a link to its source

#### Scenario: Build tooling is left out

- **WHEN** `npm run build` finishes
- **THEN** `dist/licenses/third-party-npm.html` doesn't list `caniuse-lite` or `@lingui/cli`

### Requirement: The app and the tools ship a notice for every Rust crate they link

`public/licenses/third-party-rust.html` SHALL list every crate that the desktop app, `maxima-lsp`, `maxima-dap` or `aximar-mcp` links on any platform Aximar builds for, with its name, version, licence and licence text, and a link to its source for a crate under MPL-2.0. The tools release archives SHALL include it as `THIRD-PARTY-RUST.html`.

#### Scenario: An MPL crate links its source

- **WHEN** the file is generated
- **THEN** its entry for `cssparser` names MPL-2.0 and links the crate's source on crates.io

#### Scenario: A tools archive carries the notices

- **WHEN** the tools release job packages `aximar-tools-x86_64-unknown-linux-gnu.tar.gz`
- **THEN** the archive holds `THIRD-PARTY-RUST.html` beside the binaries

### Requirement: The Rust notices stay current

CI SHALL regenerate the Rust notices and fail when the result differs from the checked-in file, naming the command that regenerates it.

#### Scenario: A new crate without a regenerated file fails

- **WHEN** a change adds a crate to `Cargo.lock` and leaves `third-party-rust.html` as it was
- **THEN** the CI check fails and says to run `npm run licences:rust`

### Requirement: Only accepted licences get in

Both generators SHALL fail when a bundled package or a linked crate has a licence, or a choice of licences, that includes none of: MIT, MIT-0, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, 0BSD, Unicode-3.0, CC0-1.0, BSL-1.0, Unlicense, OFL-1.1, MPL-2.0. Where a package offers a choice, the notice SHALL name the accepted licence it is used under. The failure SHALL name the package and its licence.

#### Scenario: A GPL crate stops the build

- **WHEN** a crate under GPL-3.0-only enters the dependency tree
- **THEN** the Rust notices fail to generate, naming the crate and GPL-3.0-only

#### Scenario: A choice is resolved to an accepted licence

- **WHEN** a crate offers "MIT OR Apache-2.0 OR LGPL-2.1-or-later"
- **THEN** its notice names MIT, the first accepted licence in the list's order

### Requirement: A user can read the notices from the app

The app SHALL show the notices in a window of their own, opened from a Licences row in the General section of Settings. On macOS and Linux the About panel SHALL say, in its credits, where to find them. The window SHALL show `licenses/index.html`, which links the font licences, the npm notices and the Rust notices. The row's text SHALL be translatable.

#### Scenario: Settings opens the notices

- **WHEN** the user presses Show licences in Settings, General
- **THEN** a window titled Licences opens on the index, and its links show each notice file

#### Scenario: Every notice has a way back

- **WHEN** the user opens the npm packages, the Rust crates or the fonts in the Licences window
- **THEN** the tabs stay along the top of the window, and a link to a package's source opens in the system browser

#### Scenario: A development build explains the missing npm notices

- **WHEN** the app runs from `npm run tauri dev` and the user opens the npm packages tab
- **THEN** the window says the npm notices come with a built app, and shows no HTML source

#### Scenario: The notices work offline

- **WHEN** the computer has no network and the user opens the notices
- **THEN** every notice shows, because every file is in the app bundle
