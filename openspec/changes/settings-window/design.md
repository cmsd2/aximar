## Context

`src/components/SettingsModal.tsx` (990 lines) renders every setting as a row in one `settings-section` div. It keeps a local copy of `AppConfig`, and its `update()` sends each change to `set_config` and applies the change to the document (CSS variables, the store, print margins). `src/App.tsx` applies the same CSS variables again at startup with its own copy of that code.

Three components (`ClaudeCodeSetup`, `CodexSetup`, `GeminiSetup`) each call a status command on mount and again whenever the address or token changes. `claude_mcp_status` and `codex_mcp_status` run `claude mcp get aximar` and `codex mcp get aximar`; `gemini_mcp_status` runs `gemini --version`, then reads `~/.gemini/settings.json`. Opening Settings therefore starts three programs, and typing in the listen address starts three more for each change.

Two other facts shape the design:

- The Codex and Gemini Configure commands already write their files with `toml_edit` and `serde_json`, without running those programs. Only Claude Code's Configure runs its CLI (`claude mcp remove`, then `claude mcp add`).
- `aximar_core::maxima::process::find_maxima_binary()` tries `AXIMAR_MAXIMA_PATH`, then three Unix paths or `C:\maxima-*`, then falls back to the bare name `maxima`. The session uses the Maxima path setting first, then this function. Nothing reports which one won.

A macOS app launched from Finder gets a short PATH (`/usr/bin:/bin:/usr/sbin:/sbin`). The current bare `Command::new("claude")` fails there for most users, and a plain PATH lookup would fail the same way.

## Goals / Non-Goals

**Goals:**

- One section of the window per kind of setting, with a hint per field.
- No AI client program runs unless the user asks for it.
- A deep-link API any part of the window can call.
- One place that applies config to the document, shared by startup and Settings.
- The Maxima status line and the session agree on which Maxima runs.

**Non-Goals:**

- New settings (cmsd2/aximar#22 is its own change).
- A separate OS window for Settings.
- Changes to MCP tools or the notebook file format.

## Decisions

### 1. A shell plus one component per section

`src/components/settings/` holds:

- `SettingsShell.tsx`, the modal, sidebar and footer;
- `sections.ts`, the list of sections;
- one file per section (`GeneralSection.tsx`, `NotebookSection.tsx`, `MaximaSection.tsx`, `PrintingSection.tsx`, `AiAssistantsSection.tsx`, `TranslationSection.tsx`);
- `SettingRow.tsx` (label, hint, control);
- `ChoiceButtons.tsx` for the button groups that repeat today.

`sections.ts` exports an ordered array of `{ id, label: MessageDescriptor, available: () => boolean, Component }`. Translation's `available` returns `TRANSLATOR_TOOLS`. The shell renders the sidebar from this array, so a new section needs one entry.

`SettingsModal.tsx` keeps its export name and re-exports the shell, so `App.tsx` and the stories change only to pass the section.

Alternative: keep one file and add headings. That keeps the 990-line file growing and gives no place for per-section state, such as the Maxima status or the AI client status. Rejected.

### 2. Section ids and the remembered section

The ids are `general`, `notebook`, `maxima`, `printing`, `ai-assistants` and `translation`. `AppConfig` gains `settings_section: String` with a default of `general`. `get_config` returns it as stored. The window maps an unknown id, or one whose section isn't available, to `general`. Rust doesn't need the list of sections, so a pre-release build that saves `translation` doesn't make a release build's config invalid.

Choosing a section in the sidebar calls `set_config({ settings_section })`. The config file already holds UI state such as `variables_open`, so this keeps one store.

Alternative: `localStorage`. The webview's storage is per origin and gets cleared with the web data, and it would add a second place for settings. Rejected.

### 3. Deep links through a small store

A Zustand store `useSettingsStore` holds `{ open: boolean, section: SectionId | null }` and an action `openSettings(section?: SectionId)`. `App.tsx` reads it in place of its `settingsOpen` state. Anything can call `openSettings("maxima")`, including hooks and the log panel. The toolbar and the menu item call `openSettings()` with no section, which shows the remembered one. A call with a section also saves it as remembered.

Callers:

- **Maxima Not Found** (`src/hooks/useMaxima.ts`) switches from `message()` to the dialog plugin's `ask()` with an "Open Settings" OK label and a "Close" cancel label. On OK it calls `openSettings("maxima")`. The last paragraph's text stops saying "configure the path in Settings", since the button does that.
- **MCP bind failure.** App log entries gain an optional `link: { kind: "settings", section }` field. `emit_app_log` in `src-tauri/src/tauri_output.rs` gets a sibling, `emit_app_log_with_link`. `src-tauri/src/mcp/startup.rs` uses it for the bind failure, and `LogPanel` renders the link as a translated "Open AI assistants settings" button. The log's existing entries are unchanged.

### 4. The menu item and keys

`src-tauri/src/menu.rs` adds a `settings` item with the accelerator `CmdOrCtrl+,` and the label `msg!("Settings...")` with a `max-length:24` note. On macOS it goes in the Aximar menu after About. Elsewhere it goes at the end of the Edit menu, after a separator. Its click emits `menu-event` with the id `settings`, as the other items do, and the window's menu handler calls `openSettings()`.

The shell handles keys on its own root:

- Esc closes it unless the event's target is an open `<select>`.
- The sidebar is a `role="tablist"` with `aria-orientation="vertical"` and a roving `tabIndex`. Up, Down, Home and End move focus and select the section.
- The page is a `role="tabpanel"` labelled by its tab.

`App.tsx`'s global shortcut handler already skips events inside `.settings-modal`.

### 5. Hints and translator notes

`SettingRow` takes `label`, `hint` and the control, and renders the hint as a `<p className="settings-hint">` with an id that the control's `aria-describedby` points at. Every hint uses `t({ message, comment })`, with a comment that starts "Settings, <section> section: hint under <label>". Sidebar labels use `msg()` with a comment ending `max-length:16`. Sixteen characters fits "AI assistants" (13) and leaves room for longer languages at the sidebar's width of 160px at 13px.

### 6. One function applies config to the document

`src/lib/apply-config.ts` exports `applyConfig(config: Partial<AppConfig>, store)`, holding the CSS-variable, store and print-margin code that `App.tsx` and `SettingsModal.update()` both hold today. Startup calls it with the whole config, and Settings calls it with each update. This is the only frontend refactor beyond the split, and the section components need it to stay small.

### 7. Text fields save on blur or Enter

Today the Docker image and Maxima path fields save on every keystroke, and the listen address saves on blur. A `TextSetting` component keeps a draft and calls `update()` on blur or Enter, and Esc inside it restores the saved value without closing the window. Keystrokes no longer write the config file. They no longer trigger the Maxima status lookup or the AI client lookup either.

### 8. Maxima status from core

`aximar_core::maxima::process` gains:

```rust
pub enum MaximaSource { Setting, Environment, StandardLocation, Path }
pub struct MaximaLocation { pub path: Option<PathBuf>, pub source: MaximaSource, pub problem: Option<MaximaProblem> }
pub enum MaximaProblem { NotFound, NotExecutable }
pub fn locate_maxima(setting: Option<&str>) -> MaximaLocation
```

It checks, in order, the setting, `AXIMAR_MAXIMA_PATH`, the standard locations, and then a PATH lookup for `maxima` (or `maxima.bat` on Windows). `find_maxima_binary()` becomes a wrapper that keeps its current return value, and the Local backend's start calls `locate_maxima`, so the status line and the session use the same order. A Tauri command `maxima_status()` reads the config and returns the location for Local. For WSL, the Maxima section keeps calling `check_wsl_maxima`, and the status line shows its result. Docker shows no status line in this change.

This logic belongs in core because the MCP server's `get_session_status` may report it later. The Tauri command only translates the result.

### 9. AI clients without running them

`src-tauri/src/commands/mcp_clients.rs` takes the MCP client code out of `config.rs`:

- `fn find_command(name) -> Option<PathBuf>` looks the name up with the `which` crate, which handles PATHEXT on Windows, over PATH plus extra directories where CLIs install: `~/.local/bin`, `~/.claude/local`, `~/.npm-global/bin`, `/opt/homebrew/bin` and `/usr/local/bin`. It returns the first executable file it finds. It runs nothing.
- `mcp_clients_status() -> McpClientsStatus` returns, per client, `installed: Option<PathBuf>` and `configured: ConfigState`. `ConfigState` is `Yes`, `No`, `Unchecked`, or `Error(String)`. Codex reads `~/.codex/config.toml` with `toml_edit` and looks for `mcp_servers.aximar`. Gemini reads `~/.gemini/settings.json` and looks for `mcpServers.aximar`. Claude is always `Unchecked`.
- `claude_mcp_check() -> ConfigState` runs `<found path> mcp get aximar` with `kill_on_drop` under a 10-second `tokio::time::timeout`.
- `claude_mcp_configure` runs the found path, not the bare name, and the window calls `claude_mcp_check` after it succeeds.
- The Codex and Gemini Configure commands keep their file-writing code and move into the same module. The window refreshes their rows by calling `mcp_clients_status` again.

The old `claude_mcp_status`, `codex_mcp_status` and `gemini_mcp_status` commands go away, and their clients in `config-client.ts` go too. No other caller uses them (the MCP server doesn't).

The window calls `mcp_clients_status` when the AI assistants section mounts and after a Configure. It doesn't call it when the address or token changes: those don't change what's installed, and a Configured client keeps its old token until the user presses Reconfigure. The row's hint says so.

Alternative: read `~/.claude.json` for Claude Code. Its layout belongs to Claude Code and has changed between versions, and project-scoped entries live elsewhere. The CLI is the supported interface. Rejected.

### 10. MCP server status

`McpController` in `src-tauri/src/mcp/controller.rs` gains a `state: Mutex<McpServerState>`, where `McpServerState` is `Off`, `Listening { address }` or `Failed { address, reason }`. `startup.rs` sets it after `bind`. A command `mcp_server_status()` returns it, and the controller emits `mcp-server-status` on each change, so the line updates when the switch restarts the server.

### What changes for MCP clients and saved notebooks

Nothing. No MCP tool or result changes, and notebooks don't store settings. The config file gains `settings_section`, which older builds ignore as an unknown field: `AppConfig` reads with defaults and doesn't deny unknown fields.

## Risks / Trade-offs

- **[Esc in a native select]** WebKit may deliver Esc to the page when a native `<select>` popup closes. → The shell ignores Esc whose target is a `<select>`. The by-hand check (task 4.4) covers macOS, Windows and Linux.
- **[PATH lookup finds a different claude than the shell]** Extra directories come after PATH, so a user whose shell PATH differs from the app's could see a different binary. → The row's hint shows the path Aximar found, and the check runs that path.
- **[Unchecked looks like a regression]** Users who saw "Configured" at once now see "Not checked" for Claude Code. → The row's Check button sits next to the status, and Configure checks afterwards. The user docs give the reason.
- **[More strings for translators]** About 30 hints and labels arrive in the catalog. → Each has a note, and the Settings stories put them in the gallery.
- **[The split changes many lines at once]** → Group 2 moves code without changing behaviour, behind the existing stories and a new test, before later groups change behaviour.
- **[Group order]** The AI client change (group 1) lands first and works with the current flat window, since it removes the program runs that set off the malware alert. The sections come after it.
