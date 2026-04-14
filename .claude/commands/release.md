---
name: release
description: Release aximar tools or GUI desktop app
argument-hint: "<tools|gui> <new-version>"
---

# Release aximar

This repo has **two independent release processes**. The first argument selects which one.

## Arguments

`$ARGUMENTS` should be `tools <version>` or `gui <version>` (e.g. `tools 0.3.0` or `gui 0.13.0`). If not provided or ambiguous, ask the user which release type and version they want.

---

## Option A: Tools release (`tools`)

Releases the standalone language tools (`maxima-lsp`, `maxima-dap`, `aximar-mcp`) consumed by the VS Code extension. CI builds are triggered by `tools-v*` tags via `.github/workflows/release-tools.yml`.

### Pre-flight checks

1. Verify on `master` branch.
2. Verify working tree is clean.
3. Read current versions from `crates/maxima-lsp/Cargo.toml` and `crates/maxima-dap/Cargo.toml` (these two always share a version) and `crates/aximar-mcp/Cargo.toml` (may differ).
4. Confirm the tag `tools-v<version>` does not already exist.

### Steps

#### 1. Bump versions

Ask the user which crates to bump. Typical options:

- **All three** — bump `maxima-lsp`, `maxima-dap`, and `aximar-mcp` to the new version.
- **LSP + DAP only** — bump `maxima-lsp` and `maxima-dap`; leave `aximar-mcp` as-is.
- **MCP only** — bump `aximar-mcp` only.

Update the `version = "..."` line in each selected `Cargo.toml`.

Then run `cargo check --workspace` to update `Cargo.lock`.

#### 2. Update CHANGELOG.md

- Read `CHANGELOG.md`. It covers tools releases only.
- Add a new `## [<version>] — <today's date>` section at the top (below the header paragraph).
- Add per-tool subsections (`### maxima-dap`, `### maxima-lsp`, `### aximar-mcp`) only for crates that changed.
- Ask the user what to put in each subsection, or suggest entries based on recent commits since the last tools tag.
- Add a release link at the bottom: `[<version>]: https://github.com/cmsd2/aximar/releases/tag/tools-v<version>`
- Show the diff and ask for confirmation.

#### 3. Commit

- Stage all modified `Cargo.toml` files, `Cargo.lock`, and `CHANGELOG.md`.
- Commit message: `Release tools v<version>`

#### 4. Tag

- `git tag tools-v<version>`

#### 5. Push

- Ask for confirmation.
- `git push && git push origin tools-v<version>`
- Remind the user that CI will create a **draft** GitHub release with platform binaries. They need to review and publish it manually.

---

## Option B: GUI release (`gui`)

Releases the Aximar desktop app (Tauri). CI builds are triggered by `v*` tags via `.github/workflows/release.yml`. The existing `tools/release.py` script automates most of this.

### Pre-flight checks

1. Verify on `master` branch.
2. Verify working tree is clean.
3. Read current version from `package.json`.
4. Confirm the tag `v<version>` does not already exist.

### Steps

#### 1. Bump versions

Update the version in all three locations (they must stay in sync):

- `src-tauri/Cargo.toml` — `[package] version`
- `package.json` — `"version"`
- `src-tauri/tauri.conf.json` — `"version"`

Also bump `crates/aximar-core/Cargo.toml` and `crates/aximar-mcp/Cargo.toml` if they match the old version.

Then run `cargo check --workspace` to update `Cargo.lock`, and `npm install` to update `package-lock.json`.

#### 2. Commit

- Stage: `src-tauri/Cargo.toml`, `crates/aximar-core/Cargo.toml`, `crates/aximar-mcp/Cargo.toml`, `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, `Cargo.lock`
- Commit message: `Bump version to <version>`

#### 3. Tag

- `git tag v<version>`

#### 4. Push

- Ask for confirmation.
- `git push && git push origin v<version>`
- Remind the user that CI will create a **draft** GitHub release with platform installers (dmg, deb, rpm, AppImage, msi, exe). They need to review and publish it manually.

---

## Summary

After completion, print:
- Release type (tools or gui)
- Old version -> new version
- Tag created
- CI workflow that will run
- Reminder to publish the draft release on GitHub
