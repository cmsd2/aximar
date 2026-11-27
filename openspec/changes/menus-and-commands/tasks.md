## 1. The command registry and shortcut names (one PR, no visible change)

- [ ] 1.1 Add `src/commands/shortcut.ts` with the neutral `Shortcut` form, `toAccelerator`, `formatShortcut(s, platform)` and `matches(s, event)`, reading the platform once; verify with vitest `shortcut.test.ts`: `formats_for_macos` (⌃⌥⇧⌘ order), `formats_for_windows_and_linux`, `accelerators_for_tauri`, `matches_a_keydown`
- [ ] 1.2 Add `src/commands/registry.ts` with every command in the spec's menu list (id, `msg()` label with translator note and max-length, menu place, shortcut or editor shortcut, enabled, checked, run), whose `run` calls today's functions (`nbUndo`, `runAll`, `restartSession`, `cancelEvaluation`, `openNewNotebook`, panel toggles, ...); verify with vitest `registry.test.ts`: `ids_are_unique`, `shortcuts_are_unique`, `every_label_has_a_note`, `run_and_run_and_stay_have_no_accelerator`
- [ ] 1.3 Replace the written-in "Cmd+..." strings in `Toolbar.tsx`, `TabBar.tsx` and `ShortcutHints.tsx` with `formatShortcut`, and build `ShortcutHints` from the registry; verify the Shortcuts story in light and dark, and vitest `shortcut-hints.test.tsx` that renders on a mocked Windows platform and finds no ⌘ or "Cmd"
- [ ] 1.4 Run `scripts/i18n-extract` and the gallery; verify `scripts/i18n-extract --check`, `npx tsc --noEmit` and `npx vitest run` pass

## 2. The menu bar from the registry (one PR)

- [ ] 2.1 Add the `MenuModel` types in `src-tauri/src/menu.rs` (submenus; custom items with id, label, accelerator, enabled, checked; predefined kinds) and build a menu from one with `CheckMenuItemBuilder` for checked items; verify with Rust tests `builds_every_predefined_kind`, `builds_a_check_item_checked`, `builds_a_disabled_item` and `an_unknown_predefined_kind_is_an_error`
- [ ] 2.2 Add the Tauri commands `set_menu(model)` and `set_menu_state(items)` (updating items in place with `Menu::get`), and emit `menu-command { id }` to the focused window for custom items; keep today's `MenuLabels` menu as the startup fallback, its File items sending the registry's ids; verify with a Rust test `state_updates_find_items_by_id` and `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] 2.3 Add `src/commands/menu-model.ts`, which turns the registry into the model in the spec's order (with the Aximar menu on macOS and Settings in Edit elsewhere), and a `useMenuSync` hook that sends the model on load and on a language switch, and the state on focus and on change; verify with vitest `menu-model.test.ts`: `menus_in_spec_order`, `every_item_is_a_command`, `macos_has_the_app_menu`, `windows_puts_settings_in_edit`, and `menu-sync.test.ts` that the state is sent on a store change without resending the model
- [ ] 2.4 Listen for `menu-command` in `App.tsx` and run the command, replacing the `menu-event` switch; verify with vitest `menu-command.test.tsx` that `file.save` and `view.docs` run their commands, and by hand on macOS that File > Save and View > Docs work

## 3. Shortcuts bound once, and Undo for the notebook (one PR, after 2)

- [ ] 3.1 Remove the window `keydown` handlers in `App.tsx` and `TabBar.tsx` for keys the menu now binds, keeping Ctrl+Tab and Ctrl+Shift+Tab; drop Mod+T and Mod+D; verify with vitest `window-keys.test.tsx` that Mod+D and Mod+T dispatch nothing and Ctrl+Tab switches tabs
- [ ] 3.2 Make Edit's Undo and Redo custom items whose commands call `nbUndo`/`nbRedo` from a cell or no field, and `document.execCommand` in other fields; verify with vitest `undo-target.test.tsx`: `undo_from_a_cell_undoes_the_notebook`, `undo_in_the_find_bar_undoes_its_text`
- [ ] 3.3 Close Tab (Mod+W) closes the window on the last tab through the existing close check; Close Window is Mod+Shift+W; verify with vitest `close-tab.test.tsx` (two tabs: one closes; one tab: the window's close is called)
- [ ] 3.4 Check by hand on macOS, Windows and Linux: each accelerator fires its command once with focus in a cell, in the find bar and nowhere; Mod+Z in a cell and in the find bar; Mod+W with one and two tabs; View's check marks following focus between two windows; record the result in this task, and decide Move Up/Down's keys (design, Open Questions)

## 4. The toolbar (one PR, after 1)

- [ ] 4.1 Add the toolbar icons to `src/components/ui/icons.ts` (`IconPlayerTrackNext`, `IconPlayerStop`, `IconRefresh`, `IconVariable`, `IconBook`, `IconListDetails`, `IconSettings`); verify the Foundations icons story lists them and `node scripts/check-css-tokens.mjs` passes
- [ ] 4.2 Rewrite `Toolbar.tsx` from the registry: Run All, Interrupt (enabled only while a cell runs), Restart, status | Variables, Docs, Log (with its badge) as tinted toggles, Settings; remove + Cell, + Markdown, Undo, Redo, Templates and the file name; verify with vitest `toolbar.test.tsx`: `shows_only_the_session_and_panels`, `interrupt_is_disabled_when_idle`, `a_toggle_is_pressed_when_its_panel_is_open`
- [ ] 4.3 Update the Window stories (toolbar idle, running, panels open; the whole window) in both themes; verify the contrast scan finds no text or icon under 4.5:1 and 3:1 respectively
- [ ] 4.4 Remove the toolbar's old strings and run the gallery and `scripts/i18n-extract`; verify `--check` passes

## 5. Commands in the palette (one PR, after 1)

- [ ] 5.1 Add a Commands group to `CommandPalette.tsx` (up to five matches above Functions, with shortcuts; `>` for commands only; disabled ones left out); verify with vitest `palette-commands.test.tsx`: `rest_finds_restart`, `greater_than_lists_only_commands`, `choosing_a_command_runs_it_and_closes`, `disabled_commands_are_left_out`
- [ ] 5.2 Add a palette story with commands; verify it in the gallery in English, Spanish and pseudo

## 6. The cell context menu (one PR, after 1)

- [ ] 6.1 Add `ContextMenu` (`role="menu"`, arrow keys, Escape, outside click and scroll close it), reusing the + menu's styles; verify with vitest `context-menu.test.tsx`: `arrows_move_between_items`, `escape_closes`
- [ ] 6.2 Open it from `Notebook.tsx` on a right-click outside `.cm-content` and the Markdown textarea, with the cell commands run for the clicked cell; verify with vitest `cell-context-menu.test.tsx`: `delete_acts_on_the_clicked_cell`, `a_right_click_in_the_editor_text_opens_no_menu`
- [ ] 6.3 Add a context menu story; verify in both themes

## 7. One icon set and one panel header (one PR, after 4)

- [ ] 7.1 Add `PanelHeader` to `src/components/ui/` and use it in the variables panel, docs panel, log window and find bar; verify the Panels and Editor stories in both themes and a `ui.test.tsx` case for its close button's accessible name
- [ ] 7.2 Replace the character icons in the cell actions, find bar and docs navigation with Tabler icons; verify with vitest `no-character-icons.test.tsx`, which renders them and fails on a button whose only content is ▲ ▼ ▶ × ← → ↔ or that has no accessible name
- [ ] 7.3 Document `IconButton` use and `PanelHeader` in `docs/design-library.md`; verify the Guide story renders the new section

## 8. Help and the Keyboard Shortcuts list (one PR, after 2)

- [ ] 8.1 Add Help's commands: Aximar Help opens the docs panel; What's New and Report an Issue open their GitHub pages with the opener plugin; verify with vitest `help-commands.test.ts` that mocks the opener and checks the URLs
- [ ] 8.2 Add the Keyboard Shortcuts dialog, built from the registry, grouped by menu, then the editor keys; verify with vitest `shortcuts-dialog.test.tsx` that every menu shortcut appears once, and a story in both themes

## 9. Docs and wrap-up

- [ ] 9.1 Mark items 25 to 33 of `docs/ui-audit.md` done or changed, with links to the PRs; verify the doc's table matches the menus as built
- [ ] 9.2 Add a Keyboard Shortcuts section to the README listing the registry's shortcuts in macOS and Windows form; verify it against the dialog
- [ ] 9.3 Run every check (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npx tsc --noEmit`, `npx vitest run`, `node scripts/check-css-tokens.mjs`, `npm run i18n:gallery`, `scripts/i18n-extract --check`) and archive the change
