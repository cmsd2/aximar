## Why

Aximar's commands live in six places built one at a time: the native menu bar, the toolbar, the tab bar, buttons on a hovered cell, keyboard shortcuts caught in the window, and a palette that searches functions. No place lists them all, many exist in only one, and the menu bar, where macOS users look for a command and learn its shortcut, holds only the file commands. The audit in `docs/ui-audit.md` ("Menus and toolbars", items 25 to 33) lists what follows: Edit's Undo isn't the notebook's undo, Cmd+W is bound twice, Cmd+N and Cmd+T do the same thing, Cmd+D deletes a cell with no warning, labels say Cmd on Windows and Linux, the toolbar shows thirteen controls at one weight, and icons come in three styles. The insertion caret (cmsd2/aximar#117) already took inserting cells out of the toolbar, which makes this the time to give the rest one model.

## What Changes

- **One list of the window's commands**, each with an id, a translatable label, an optional shortcut, and when it is enabled or checked. The menu bar, the palette, the cell context menu, the toolbar's tooltips, the shortcut hints and a Keyboard Shortcuts list are all built from it.
- **A menu bar that lists every command with its shortcut**: File, Edit, Cell, Session, View, Window and Help, plus the Aximar menu on macOS. The window sends the menu to the backend, which builds the native menu; a menu item runs its command in the focused window.
- **Edit > Undo and Redo undo the notebook** (cells added, deleted, moved, edited), and still undo the text in fields outside the cells, such as the find bar.
- **Shortcuts:** each command's shortcut is bound once, by the menu. Cmd+W closes the tab (the window, on its last tab) and Cmd+Shift+W closes the window. Cmd+T is dropped, as Cmd+N opens a new notebook tab. **BREAKING** for muscle memory: Delete Cell moves from Cmd+D to Cmd+Shift+Backspace, and Cmd+T no longer opens a tab.
- **Shortcut labels follow the platform:** ⌘ ⇧ ⌥ on macOS, Ctrl+Shift+Alt elsewhere, in menus, tooltips, hints and the palette.
- **A smaller toolbar:** Run All, Interrupt and Restart beside the session status on the left; Variables, Docs and Log as toggles and Settings on the right, all icon buttons with tooltips. + Cell, + Markdown, Undo, Redo, Templates and the notebook's name leave the toolbar; they are in the menu bar, the gaps and the tab. **BREAKING** for users who rely on those toolbar buttons.
- **Commands in the palette:** the palette lists matching commands above functions, and `>` shows only commands.
- **A context menu on cells** with the Cell menu's commands for that cell.
- **One icon set:** every icon button uses a Tabler icon through `src/components/ui/icons.ts`, with one close button and one panel header for the variables, docs and log panels and the find bar.

## Non-goals

- Changing what any command does to a notebook. Undo, run, move and delete keep going through the backend's notebook commands (ADR 007); this change only changes where they are reached from.
- A customisable toolbar or user-defined shortcuts.
- A hard interrupt of Maxima. Interrupt stays the cooperative cancel the stop button sends today; Restart remains the way to stop a computation that doesn't check for it.
- Menus in the VS Code extension or over MCP. MCP has no menus, and its tools are unchanged.
- The notebook's own look (output well, errors, notes), which the audit lists separately.

## Capabilities

### New Capabilities

- `window-commands`: the window's list of commands and every place that shows it: the menu bar, shortcuts and their labels, the toolbar, the palette's commands, the cell context menu, the Keyboard Shortcuts list, and the shared icon buttons and panel headers.

### Modified Capabilities

None. The settings-window change, not yet archived, already describes Settings' menu item and Cmd+,; this change keeps both.

## Impact

- **Frontend:** a command registry and dispatcher (new), `Toolbar.tsx` (rewritten), `TabBar.tsx` and `App.tsx` (their keydown handlers move into the registry), `CommandPalette.tsx` (a commands group), `ShortcutHints.tsx` (built from the registry), `Notebook.tsx` and the cells (context menu), the find bar and the panels (shared header and close button), `ui/icons.ts` (more icons), and the catalogs (new strings, removed toolbar strings).
- **Tauri:** `src-tauri/src/menu.rs` builds the menu from a model the window sends, keeps a small built-in menu until the first window has loaded, and sends `menu-command` events; a command to update checked and enabled state when the focused window changes. The localisation change's task 3.3 (menus from the catalog) then covers only that built-in menu.
- **Core and MCP:** none.
- **Tools:** none.
- **Docs:** `docs/ui-audit.md` items 25 to 33, `docs/design-library.md` (icon buttons, panel header), and a Keyboard Shortcuts section for the README or the docs.
- **Issues:** none filed; the source is the audit. Part of stream S7 in `docs/project-plan.md`.
