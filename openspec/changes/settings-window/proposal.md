## Why

The Settings window is one flat list of about twenty rows in no order: language sits above theme, the backend rows sit between argument help and font size, and the MCP rows sit above the variables panel checkbox. Rows have no explanation, so a new user can't tell "Argument help: Hint / Snippet / Active hint" apart without trying each. Opening the window also runs `claude mcp get aximar` and `codex mcp get aximar` at once, and a stale Codex binary signed with a revoked certificate set off a macOS malware alert the moment a user opened Settings. The localisation work (messages-and-localisation) adds a language picker and two translator rows, which makes the list longer still, so the window needs a structure before more settings arrive (such as the kernel-events toggle in cmsd2/aximar#22).

## What Changes

- The Settings window gets a sidebar of sections, each with its own page:
  - **General**: language, theme, variables panel open at start.
  - **Notebook**: font size, cell style, argument help, suggestions under results (added for cmsd2/aximar#44), Markdown font, Markdown indent.
  - **Maxima**: backend, then only the fields for the chosen backend (Maxima path for Local; container engine and image for Docker; distro for WSL), a status line saying which Maxima Aximar found or why it found none, and the eval timeout.
  - **Printing**: print font size and the four margins.
  - **AI assistants**: the MCP server switch, a status line for the server, one row per client (Claude Code, Codex, Gemini CLI), and the listen address and token under an Advanced disclosure.
  - **Translation**: only in pre-release builds, holding the existing translation preview and show-sources rows.
- Every field shows a one-line hint under its label.
- Settings apply as soon as they change, as today. The footer keeps only a Close button.
- Other parts of the window can open Settings at a given section. The Maxima Not Found dialog gains an Open Settings button that opens Maxima, and an MCP server that fails to bind its address reports it in the log with a link to AI assistants.
- The window reopens at the section it last showed.
- Cmd+, (Ctrl+, elsewhere) opens Settings from a new menu item, Esc closes it, and the arrow keys move through the sidebar.
- All new text goes through Lingui with translator notes. Sidebar labels carry a `max-length` note, which the existing catalog test enforces.
- **BREAKING** for the Claude Code row, which no longer shows its status at once: opening Settings no longer runs any AI client's program. Aximar finds installed clients by looking them up on PATH, reads `~/.codex/config.toml` and `~/.gemini/settings.json` to see whether they list Aximar, and runs `claude mcp get aximar` only when the user presses Check or after Configure. Claude Code's CLI is the only supported way to read its MCP configuration, so Aximar keeps using it, on request.

## Capabilities

### New Capabilities

- `settings-window`: the Settings window's sections, layout, hints, apply-at-once behaviour, deep links, remembered section, keyboard use and the localisation of its text.
- `ai-assistant-setup`: how Aximar detects AI clients and whether they list its MCP server, when it may run their programs, and how Configure works.

### Modified Capabilities

None. No existing spec covers the Settings window or MCP client setup.

## Impact

- **Frontend**: `src/components/SettingsModal.tsx` splits into a shell with a sidebar and one component per section under `src/components/settings/`. `src/App.tsx` holds the open section. `src/hooks/useMaxima.ts` (Maxima Not Found dialog), `src/components/LogPanel.tsx` (entry links), `src/lib/config-client.ts`, `src/styles/global.css`, and the Settings stories in `src/stories/`.
- **Tauri**: `src-tauri/src/commands/config.rs` replaces the three `*_mcp_status` commands with one `mcp_clients_status` that runs nothing, adds `claude_mcp_check`, adds a `settings_section` config field, and adds a `maxima_status` command. `src-tauri/src/menu.rs` adds the Settings menu item. `src-tauri/src/mcp/` records why the server failed to start.
- **Core**: `crates/aximar-core` gains a function that resolves which Maxima the Local backend would run and from where, so the status line and the session agree.
- **MCP**: no change to tools or their results.
- **Catalog**: new strings and `max-length` notes in `locales/messages.pot` and the catalogs.
- **Docs**: `docs/i18n.md` (max-length notes) and a Settings section in the user docs. `CHANGELOG.md` covers only the tools, so it doesn't change.
- **Issues**: makes room for cmsd2/aximar#22 (kernel-events toggle) in the Maxima section, without closing it.

## Non-goals

- A search box over settings. Six sections of at most six rows each don't need one.
- Moving Settings into its own OS window. It stays a modal inside the main window.
- An OK/Cancel model with pending changes.
- New settings. The kernel-events toggle (cmsd2/aximar#22) and any others come in their own changes.
- Configuring AI clients other than Claude Code, Codex and Gemini CLI.
- Changing how Configure writes Codex's or Gemini's files.
