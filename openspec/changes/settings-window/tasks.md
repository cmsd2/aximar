## 1. AI clients without running them (one PR, lands first)

- [x] 1.1 Move the MCP client commands from `src-tauri/src/commands/config.rs` into `src-tauri/src/commands/mcp_clients.rs` without changing them; verify `cargo test -p aximar` passes and the window's three rows still work
- [x] 1.2 Add `find_command(name)` over PATH plus the extra install directories, using the `which` crate; verify with unit tests `finds_a_command_on_path`, `finds_a_command_in_an_extra_directory` and `a_missing_command_is_none`, each using a temporary directory with an executable file
- [x] 1.3 Add `mcp_clients_status`, reading `~/.codex/config.toml` and `~/.gemini/settings.json` from a home directory passed in, with Claude always `Unchecked`; verify with unit tests `codex_configured_from_its_file`, `codex_without_a_file_is_not_configured`, `a_broken_gemini_file_is_an_error` and `claude_is_unchecked`
- [x] 1.4 Add `claude_mcp_check` with a 10-second timeout, and make `claude_mcp_configure` run the found path; verify with a test that points it at a stub script that sleeps 20 seconds and expects the timeout result (`#[cfg(unix)]`)
- [x] 1.5 Remove `claude_mcp_status`, `codex_mcp_status` and `gemini_mcp_status` and their clients in `src/lib/config-client.ts`; switch the three rows to `mcp_clients_status`, a Check button for Claude Code, and a check after Configure; verify `npm run lint`, `npm test` and `cargo clippy --workspace --all-targets -- -D warnings` are clean
- [x] 1.6 Add a vitest test `settings-ai-clients.test.tsx` that mocks `invoke`, opens Settings and asserts only `mcp_clients_status` was called, and that pressing Check calls `claude_mcp_check` once
- [ ] 1.7 Verify by hand that opening Settings on macOS starts no `claude`, `codex` or `gemini` process (`ps` while the window is open)

## 2. Sections, with no change in behaviour (one PR)

- [x] 2.1 Add `src/lib/apply-config.ts` with the document-styling code from `App.tsx` and `SettingsModal.update()`, and use it in both; verify with a vitest test `apply-config.test.ts` that checks the CSS variables and print margins it sets
- [x] 2.2 Add `settings_section` to `AppConfig` with a default of `general`, saved by `set_config`; verify with config tests `settings_section_defaults_to_general` and `an_older_file_without_settings_section_reads`
- [x] 2.3 Add `src/components/settings/` with `SettingsShell`, `sections.ts`, `SettingRow`, `ChoiceButtons`, `TextSetting` and the six section components, moving every row from `SettingsModal.tsx`; keep `SettingsModal` as a re-export; verify with a vitest test `settings-sections.test.tsx` that renders each section and checks the full list of config fields appears once across them
- [x] 2.4 Keep the Maxima section's backend fields to the chosen backend, as the flat window does; verify with vitest tests `docker_shows_its_fields` and `switching_back_keeps_the_path`
- [ ] 2.5 Lay out the sidebar and page in `src/styles/global.css` for both themes and the print stylesheet's exclusion; replace the Settings story with one story per section in `src/stories/Settings.stories.tsx`; verify `npm run build-storybook` and `npm run i18n:gallery` succeed and the gallery shows each section in English, Spanish and pseudo
- [x] 2.6 Hide Translation in release builds; verify with a vitest test that mocks `TRANSLATOR_TOOLS` off and on and checks the sidebar's entries
- [x] 2.7 Run `scripts/i18n-extract`, commit the template and catalogs, and verify `scripts/i18n-extract --check` and the catalog test pass

## 3. Hints and status lines (one PR)

- [x] 3.1 Write a one-line hint for every setting, with translator notes, and give each sidebar label a `max-length:16` note; verify with a vitest test that every `SettingRow` in every section has a non-empty hint, and with the catalog test after `scripts/i18n-extract`
- [x] 3.2 Add `locate_maxima` and `MaximaLocation` to `aximar_core::maxima::process`, make `find_maxima_binary` and the Local backend's start use it; verify with core tests `the_setting_wins`, `the_environment_comes_next`, `a_setting_that_names_no_file_is_a_problem` and `nothing_found`, each with a temporary directory and a controlled environment
- [ ] 3.3 Add the `maxima_status` Tauri command and the Maxima section's status line, including the WSL result from `check_wsl_maxima`; verify with a vitest test that mocks each `MaximaLocation` case and checks the line's text, and by hand that a wrong path shows "doesn't exist" in the app
- [x] 3.4 Record `McpServerState` in `McpController`, set it in `startup.rs`, add `mcp_server_status` and the `mcp-server-status` event, and show the status line; verify with a Tauri unit test `a_bind_failure_is_recorded` that binds the port first, and a vitest test of the line's three states
- [x] 3.5 Make `TextSetting` save on blur or Enter and restore the saved value on Esc, for the Maxima path, Docker image and listen address; verify with vitest tests `typing_does_not_save_until_blur`, `enter_saves` and `esc_restores_without_closing`
- [x] 3.6 Leave the new entries in `locales/es.po` for translators, as the rest of that catalog is (it has no translations yet); verify the pseudo-locale stories show no English in Settings apart from product names, commands and paths

## 4. Deep links, the remembered section and keys (one PR)

- [x] 4.1 Add `useSettingsStore` with `openSettings(section?)`, use it in `App.tsx` and the toolbar, and remember the section through `settings_section`; verify with vitest tests `opens_at_the_remembered_section`, `a_named_section_becomes_remembered` and `an_unavailable_section_falls_back_to_general`
- [x] 4.2 Switch the Maxima Not Found dialog to `ask()` with Open Settings, opening the Maxima section; verify with a vitest test that mocks the dialog plugin's `ask` to return true and checks the store opened `maxima`
- [x] 4.3 Add the optional `link` field to app log entries, `emit_app_log_with_link`, its use for the MCP bind failure, and the link button in `LogPanel`; verify with a vitest test that renders a log entry with a link and checks that clicking it opens `ai-assistants`
- [ ] 4.4 Add the Settings menu item with `CmdOrCtrl+,` in `src-tauri/src/menu.rs` (the Aximar menu on macOS, the Edit menu elsewhere), handled in the window's `menu-event` listener; verify with a `menu.rs` test that the labels include Settings, and by hand on macOS, Windows and Linux that the shortcut opens Settings and Esc in an open select menu closes only the menu
- [x] 4.5 Make the sidebar a vertical tablist with a roving tab index, Up, Down, Home and End, and Esc closing the window; verify with vitest tests `arrows_move_through_the_sidebar`, `home_and_end_jump` and `esc_closes`

## 5. Docs (with group 4's PR)

- [ ] 5.1 Describe the sections, the deep links and the AI client checks in the user docs under `docs/`, and add the Settings sidebar's `max-length` limit to `docs/i18n.md`; verify the links in the new text resolve
- [ ] 5.2 Leave CHANGELOG.md alone, since it covers only the `tools-v*` releases and none of these changes reach the tools, and verify `openspec validate settings-window` passes before archiving
