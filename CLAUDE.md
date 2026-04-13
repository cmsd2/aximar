## Project rules

- When making significant changes, update the relevant docs in `docs/`.
- When writing or editing Maxima `.mac` files, refer to [`docs/maxima-mac-syntax.md`](docs/maxima-mac-syntax.md) for syntax rules and common pitfalls.

## Releases

- Tool releases use the `tools-v*` tag convention (e.g. `tools-v0.2.0`). Tauri desktop app releases use `v*`.
- Maintain `CHANGELOG.md` in the repo root using [Keep a Changelog](https://keepachangelog.com/) style. When cutting a tools release, add an entry there first, then copy the relevant section to the GitHub Release description. The VS Code extension links to the release page via a "What's New" button in the update notification.
