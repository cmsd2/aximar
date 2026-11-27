## Context

See proposal.md for why. The requirements are in `specs/window-commands/spec.md`.

Today each surface keeps its own idea of the commands:

- `src-tauri/src/menu.rs` builds the native menu in Rust: Aximar, File, Edit and Window, with labels from the Rust catalog (`MenuLabels`, read with rspolib). File items send `menu-event` with their id to the focused window, and `App.tsx` switches on that id. Edit's Undo and Redo are Tauri's predefined items, which act on the focused text.
- Keyboard shortcuts are `keydown` listeners: `App.tsx` handles Mod+Z, Mod+Shift+Z, Mod+Y, Mod+F, Mod+Shift+F, Mod+D, Mod+Shift+Up/Down and Mod+K; `TabBar.tsx` handles Mod+T, Mod+W, Mod+Shift+[ ], Ctrl+Tab. The cell editor (CodeMirror) handles Shift+Enter, Mod+Enter and, since cmsd2/aximar#117, the arrow keys at a cell's edge.
- `Toolbar.tsx` draws 13 text buttons; `ShortcutHints.tsx` keeps its own list of keys; `CommandPalette.tsx` searches functions, packages and symbols; cell actions, the find bar and the panels draw characters (▲ ▼ ▶ × ← →) as icons. Strings like "Cmd+Z" are written into the code.
- Notebook changes (add, delete, move, undo, redo) are backend `NotebookCommand`s with undo in aximar-core (ADR 007). `cancelEvaluation()` sends a cooperative cancel that works only when kernel events are on and the running code checks for it.
- Tauri 2.11's menu API has `CheckMenuItem` with `set_checked` and `set_enabled`, normal items with `set_enabled`, predefined items, and `Menu::get(id)` to find an item later.

Layers: frontend and Tauri only. Nothing moves into or out of aximar-core: the commands that change a notebook keep calling the same Tauri commands, which keep calling the same core `NotebookCommand`s, so the GUI and MCP still behave the same for every notebook operation. MCP clients see no change; saved notebooks don't change, as nothing here is stored in the file.

## Goals / Non-Goals

**Goals:**

- One place in the frontend that defines a command, from which every surface (menu bar, palette, context menu, toolbar tooltips, hints, Keyboard Shortcuts list) is built, with tests that hold them together.
- Each shortcut handled by exactly one mechanism.
- Platform-correct shortcut names everywhere.

**Non-Goals:**

- A command system in aximar-core or exposed over MCP. These are window commands (open a panel, focus a cell, close a tab); the notebook operations behind them already live in core.
- Making menus configurable, or a plugin API for commands.
- Replacing CodeMirror's keymap; the editor keeps its own keys.

## Decisions

### 1. The registry lives in the frontend, and the frontend sends the menu to Rust

A command is a record in `src/commands/registry.ts`:

```ts
interface Command {
  id: CommandId;                 // "cell.run_all", "view.docs", ...
  label: MessageDescriptor;      // Lingui msg(), with a translator note and max-length
  menu?: MenuPlace;              // menu and group, for ordering and separators
  shortcut?: Shortcut;           // "Mod+Shift+W": neutral form, see 4
  editorShortcut?: Shortcut;     // shown, never bound (Shift+Enter, Mod+Enter)
  enabled?: (s: CommandState) => boolean;
  checked?: (s: CommandState) => boolean;
  run: (ctx: CommandContext) => void | Promise<void>;  // ctx.cellId for the context menu
}
```

`CommandState` is read from the Zustand stores (session status, undo/redo flags, open panels, active cell, tab count). Everything else derives from this list: the menu model, the palette's Commands group, the context menu, toolbar tooltips, `ShortcutHints`, and the Keyboard Shortcuts dialog.

The window serialises a menu model, a tree of submenus whose leaves are either `{ id, label, accelerator?, enabled, checked? }` or `{ predefined: "cut" | "copy" | "paste" | "select_all" | "services" | "hide" | "hide_others" | "show_all" | "quit" | "about" | "minimize" | "maximize" | "separator" }`, and calls a new Tauri command `set_menu(model)`. `menu.rs` builds it with `MenuBuilder`, `SubmenuBuilder`, `MenuItemBuilder`, `CheckMenuItemBuilder` and the predefined items, and `app.set_menu` replaces the current one. A click on a custom item emits `menu-command { id }` to the focused window, which runs the command from its registry.

*Alternatives.* Keep the menu in Rust and duplicate the list in TypeScript for the palette and context menu: two lists to keep in step, labels in two catalogs' worth of code, and the handlers are in the frontend anyway. Draw a menu bar in HTML: loses the macOS menu bar, which is where Mac users look, and the system's keyboard handling for it.

**Startup.** Until a window has loaded and sent its model, Rust keeps building today's menu from `MenuLabels` (the localisation change's task 3.3 covers translating it). It has the file commands, Edit's text items and the Aximar menu, so the app is usable while the window loads; its File items send the same command ids (`file.new`, `file.open`, ...) as `menu-command`.

**State.** On `focus`, and whenever a `checked` or `enabled` value changes, the focused window calls `set_menu_state([{ id, enabled, checked }])`, which updates items in place with `Menu::get(id)`, so the menu isn't rebuilt on every keystroke. On a language switch the window sends the whole model again.

### 2. Shortcuts are menu accelerators only

Every command with a `shortcut` gets it as the menu item's accelerator, and the window's own `keydown` listeners for those keys are removed (`App.tsx`'s Mod+Z/Y/F/D/K and moves, `TabBar.tsx`'s Mod+T/W/[ ]). One mechanism means one firing: today a menu accelerator and a window listener can both see the same key.

Kept outside the menu:

- Editor keys (Shift+Enter, Mod+Enter, the arrows at a cell's edge, Escape) stay in CodeMirror and the Markdown textarea. A menu accelerator would take Shift+Enter from every text field. Run and Run and Stay appear in the Cell menu without an accelerator; their `editorShortcut` shows in the palette and the shortcuts list.
- Ctrl+Tab and Ctrl+Shift+Tab stay a window `keydown` alias for Next and Previous Tab, as a menu item has one accelerator.

Changed bindings: Close Tab Mod+W, Close Window Mod+Shift+W (a custom item calling the window's close, which runs the existing unsaved-changes check in `onCloseRequested`), Mod+T dropped, Delete Cell from Mod+D to Mod+Shift+Backspace (CodeMirror binds Mod+Backspace, not Mod+Shift+Backspace), Interrupt Mod+., Keyboard Shortcuts Mod+/. Close Tab on the last tab closes the window.

### 3. Undo and Redo are custom items that choose their target

Edit's Undo and Redo become custom items with Mod+Z and Mod+Shift+Z. The dispatcher looks at `document.activeElement`: inside a cell (CodeMirror's content or a Markdown cell's textarea), or nowhere in a text field, it calls `nbUndo`/`nbRedo`, the backend's undo as today; in another `input` or `textarea` (find bar, palette, docs search, Settings) it calls `document.execCommand("undo" | "redo")` so the field's own history applies. Cut, Copy, Paste and Select All stay predefined, as they must reach the web view's clipboard handling.

*Alternative.* Keep the predefined items and catch Mod+Z in the window: the menu item and the key then do different things, which is the bug the audit found.

### 4. Shortcuts are written once, in a neutral form

`Shortcut` is a string like `"Mod+Shift+W"`, `"Mod+."`, `"Mod+Shift+Backspace"`. Three functions read it:

- `toAccelerator(s)` → Tauri's `"CmdOrCtrl+Shift+W"`.
- `formatShortcut(s, platform)` → `"⇧⌘W"` on macOS (symbols in Apple's order ⌃⌥⇧⌘), `"Ctrl+Shift+W"` on Windows and Linux.
- `matches(s, event)` → for the one remaining window alias and for tests.

`platform` comes from `@tauri-apps/plugin-os` if present, else `navigator.userAgent`, read once. Every written-in "Cmd+..." string (`Toolbar`'s undo tooltip, `TabBar`'s new-tab tooltip, `ShortcutHints`) goes through `formatShortcut`. Translator notes say the shortcut is a placeholder.

### 5. The toolbar is built from commands

`Toolbar.tsx` renders `IconButton`s (the design library's `Button` with `icon` and `label`, `variant="quiet"`) for `cell.run_all`, `session.interrupt`, `session.restart`, then the status, then a spacer, then `view.variables`, `view.docs`, `view.log` as toggles (`aria-pressed`, a tint from `--accent` at 12%, not a solid fill) and `app.settings`. Tooltips are `label` plus `formatShortcut`. Icons, added to `src/components/ui/icons.ts`: `IconPlayerTrackNext`, `IconPlayerStop`, `IconRefresh`, `IconVariable`, `IconBook`, `IconListDetails`, `IconSettings`. The Log badge stays, positioned on the icon. + Cell, + Markdown, Undo, Redo, Templates and the file name are removed; their strings leave the catalog.

### 6. The palette gains a Commands group

`CommandPalette.tsx` matches the query against enabled commands' labels with the fuzzy matcher it already uses and shows up to five in a Commands group above Functions, each with its shortcut. A leading `>` shows only commands, all of them when nothing follows. The palette's existing modes (`\` for symbols, package browsing) are unchanged.

### 7. The cell context menu is a React popover

`Notebook.tsx` handles `contextmenu` on a cell: if the target is inside `.cm-content` or a Markdown cell's textarea it does nothing, so the system's text menu opens; otherwise it prevents the default and opens a `ContextMenu` at the pointer with the cell commands, run with `ctx.cellId` set to that cell, not the active one. The menu is a `role="menu"` list with arrow-key navigation, closed by Escape, a click outside, or a scroll. It reuses the + menu's styles from `CellGap`.

*Alternative.* A native context menu through Tauri's `Menu::popup`: native look, but the items' state and handlers would make the round trip to Rust for every right-click, and Storybook couldn't show it.

### 8. One icon button and one panel header

A `PanelHeader` (`src/components/ui/PanelHeader.tsx`: a title or controls slot on the left, actions and one close `IconButton` with `IconX` on the right, one height) is used by the variables panel, the docs panel, the log window and the find bar. Cell actions use `IconChevronUp`, `IconChevronDown`, `IconPlayerPlay`, `IconPlayerStop`, `IconX`; the find bar `IconLetterCase`, `IconChevronUp`, `IconChevronDown`, `IconReplace`; the docs panel `IconArrowLeft`, `IconArrowRight`. A test renders these and fails on a button whose only content is one of the old characters, or an icon-only button without an accessible name.

### 9. Help

Aximar Help opens the docs panel. What's New and Report an Issue open `https://github.com/cmsd2/aximar/releases` and `https://github.com/cmsd2/aximar/issues/new` with `@tauri-apps/plugin-opener`, already a dependency. Keyboard Shortcuts opens a dialog (the design library's `Dialog`) listing the registry's commands with shortcuts grouped by menu, then the editor keys.

### 10. Testing

- Registry tests: ids unique, shortcuts unique, every command has a label with a translator note, every menu item's id is a command, `formatShortcut` for each platform, `toAccelerator` for each shortcut.
- Rust: `menu.rs` builds a menu from a sample model with every predefined kind, a check item and a disabled item; unknown predefined kinds are an error, not a panic.
- Dispatcher tests: undo goes to `nbUndo` from a cell and to `execCommand` from the find bar; Close Tab on the last tab closes the window; context-menu commands act on the clicked cell.
- Storybook: the toolbar (idle, running, panels open), the palette with commands, the context menu, the Keyboard Shortcuts dialog, the panels' shared header.
- By hand on macOS, Windows and Linux: each accelerator fires its command once, Mod+Z in a cell and in the find bar, Mod+W with one and two tabs, check marks following focus between two windows.

## Risks / Trade-offs

- [A menu accelerator may not reach the menu while a text field has focus, or may reach both the field and the menu, differently per platform] → the hand checks in tasks; any key that misbehaves on a platform keeps a window `keydown` handler there, recorded in the registry as `bindInWindow`.
- [`document.execCommand("undo")` is deprecated] → it is still supported by WebKit, WebView2 and WebKitGTK; if it stops working, a field's own undo can be kept by leaving Mod+Z unbound in those fields' `keydown`.
- [Mod+Shift+Up/Down moves the cell but, in CodeMirror on macOS, also selects to the start or end of the document] → the move wins today because the window listener runs; with menu accelerators the editor may see the key first. See Open Questions.
- [Removing toolbar buttons hides commands from people who never open menus] → the gaps' + and caret, the empty notebook, the palette and the shortcut hints cover inserting and undoing; Templates is File > New from Template and in the palette.
- [Rebuilding the native menu on language change while a menu is open] → the model is sent only on language change and window load, not on state changes; state uses `set_menu_state`.
- [Interrupt does nothing when kernel events are off or the code doesn't check for cancels] → the tooltip says so in its translator note, and Restart sits beside it; a hard interrupt is out of scope.

## Migration Plan

No data migrates. Users lose Mod+T (use Mod+N) and Mod+D (use Mod+Shift+Backspace); the desktop release's notes (the GitHub release for its `v*` tag; `CHANGELOG.md` covers only the tools) say so. Rollback is reverting the change; the Rust fallback menu means a window that fails to send its model still has a working menu.

## Open Questions

- Whether Move Up/Down keeps Mod+Shift+Up/Down or moves to Mod+Alt+Up/Down, decided after the hand check on macOS shows whether CodeMirror takes the key first. Either way the registry changes in one place.
