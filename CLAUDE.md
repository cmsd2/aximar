## Project rules

- When making significant changes, update the relevant docs in `docs/`.
- When writing or editing Maxima `.mac` files, refer to [`docs/maxima-mac-syntax.md`](docs/maxima-mac-syntax.md) for syntax rules and common pitfalls.

## Releases

- Tool releases use the `tools-v*` tag convention (e.g. `tools-v0.2.0`). Tauri desktop app releases use `v*`.
- When cutting a release, write a meaningful changelog in the GitHub Release description. List user-facing changes, bug fixes, and breaking changes — not just commit messages. The VS Code extension links to the release page via a "What's New" button in the update notification.
