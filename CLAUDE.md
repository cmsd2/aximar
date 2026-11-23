## Project rules

- When making significant changes, update the relevant docs in `docs/`.
- When writing or editing Maxima `.mac` files, refer to [`docs/maxima-mac-syntax.md`](docs/maxima-mac-syntax.md) for syntax rules and common pitfalls.
- Before every commit, these must pass with no warnings. Fix the cause; don't add `#[allow]`. CI uses the latest stable Rust, whose clippy adds lints, so run `rustup update stable` first.
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets`
  - `npm run build` when the frontend changed (it runs `tsc`)
- A branch from before the workspace was formatted conflicts on formatting: rebase, run `cargo fmt --all`, then resolve what's left.

## Releases

- Tool releases use the `tools-v*` tag convention (e.g. `tools-v0.2.0`). Tauri desktop app releases use `v*`.
- Maintain `CHANGELOG.md` in the repo root using [Keep a Changelog](https://keepachangelog.com/) style. When cutting a tools release, add an entry there first, then copy the relevant section to the GitHub Release description. The VS Code extension links to the release page via a "What's New" button in the update notification.
