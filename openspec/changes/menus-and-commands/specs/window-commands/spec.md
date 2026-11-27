## Purpose

The window's commands, from saving a notebook to opening a panel, kept in one list that the menu bar, the toolbar, the palette, the cell context menu and the shortcut lists all show, so each command can be found in the places a user looks and has one shortcut.

## ADDED Requirements

### Requirement: The menu bar lists every command

The menu bar SHALL hold these menus, items and shortcuts. "Mod" is Cmd on macOS and Ctrl elsewhere; "Alt" is Option on macOS.

- Aximar (macOS only): About Aximar, Settings... (Mod+,), Services, Hide Aximar, Hide Others, Show All, Quit Aximar.
- File: New Notebook (Mod+N), New Window (Mod+Shift+N), New from Template..., Open... (Mod+O), Save (Mod+S), Save As... (Mod+Shift+S), Export as LaTeX... (Mod+Shift+E), Print... (Mod+P), Close Tab (Mod+W), Close Window (Mod+Shift+W).
- Edit: Undo (Mod+Z), Redo (Mod+Shift+Z), Cut, Copy, Paste, Select All, Find (Mod+F), Find and Replace (Mod+Shift+F), and on Windows and Linux Settings... (Mod+,).
- Cell: New Code Cell, New Markdown Cell, Run, Run and Stay, Run All, Move Up (Mod+Shift+Up), Move Down (Mod+Shift+Down), Delete Cell (Mod+Shift+Backspace).
- Session: Interrupt (Mod+.), Restart.
- View: Variables, Docs and Log, each with a check mark while its panel is open, and Command Palette (Mod+K).
- Window: Minimize (Mod+M), Zoom, Next Tab (Mod+Shift+]), Previous Tab (Mod+Shift+[), Move Tab to New Window.
- Help: Aximar Help, Keyboard Shortcuts (Mod+/), What's New, Report an Issue.

Run (Shift+Enter) and Run and Stay (Mod+Enter) SHALL appear without a shortcut in the menu, because the cell editor handles those keys and a menu shortcut would take them from every text field; the palette and the Keyboard Shortcuts list SHALL still show their keys. Every label SHALL be translatable and SHALL follow the window's language when it changes.

#### Scenario: Run All is in the Cell menu

- **WHEN** the user opens the Cell menu
- **THEN** it lists Run All, and choosing it runs every code cell of the focused window's notebook in order, as the toolbar's Run All does

#### Scenario: The menu follows the language

- **WHEN** the user switches Aximar to Spanish
- **THEN** the menu bar's menus and items show the Spanish catalog's labels, and English where the catalog has none

#### Scenario: Labels and shortcuts come from one list

- **WHEN** the tests build the menu model and the palette's command list
- **THEN** every menu item has a command in the list, every command has a label, no two commands share a shortcut, and no two commands share an id

### Requirement: Each shortcut is bound once

A command's shortcut SHALL run that command once, whichever part of the window has focus, apart from keys a text field or the cell editor uses for editing. Mod+W SHALL close the active tab, and close the window when it is the last tab, asking about unsaved changes as closing does now; Mod+Shift+W SHALL close the window. Mod+T SHALL no longer open a tab, and Mod+D SHALL no longer delete a cell. Ctrl+Tab and Ctrl+Shift+Tab SHALL also switch to the next and previous tab.

#### Scenario: Mod+W closes one tab

- **WHEN** a window has two tabs and the user presses Mod+W
- **THEN** the active tab closes, the other stays, and the window stays open

#### Scenario: Mod+W on the last tab closes the window

- **WHEN** a window has one tab with unsaved changes and the user presses Mod+W
- **THEN** Aximar asks to save, and the window closes after Save or Don't Save

#### Scenario: Mod+D leaves the cell alone

- **WHEN** a cell has focus and the user presses Mod+D
- **THEN** no cell is deleted

### Requirement: Undo and Redo undo the notebook

Edit > Undo and Redo, and their shortcuts, SHALL undo and redo the notebook's last change (a cell added, deleted, moved or edited), as the toolbar's Undo did. In a text field outside the cells, such as the find bar, a search field or Settings, they SHALL undo the text instead. Redo SHALL be greyed out when there is nothing to redo, and Undo when there is nothing to undo.

#### Scenario: Undo from the menu restores a deleted cell

- **WHEN** the user deletes a cell and chooses Edit > Undo
- **THEN** the cell is back, in its place

#### Scenario: Undo in the find bar undoes the typing

- **WHEN** the user types in the find bar and presses Mod+Z there
- **THEN** the find bar's text goes back, and the notebook is unchanged

### Requirement: Menus act on the focused window

A menu command SHALL act on the focused window's active notebook only. The View menu's check marks and the greyed items SHALL show the focused window's state, and SHALL change when another window takes focus.

#### Scenario: Check marks follow the window

- **WHEN** one window has the docs panel open, another has it closed, and the user moves between them
- **THEN** View > Docs is checked while the first window has focus and unchecked while the second has it

### Requirement: Shortcut labels follow the platform

Wherever Aximar shows a shortcut (menus, tooltips, the shortcut hints, the palette, the Keyboard Shortcuts list), it SHALL use the platform's names: ⌘, ⇧, ⌥ and ⌃ on macOS, and Ctrl, Shift and Alt joined by + on Windows and Linux.

#### Scenario: A tooltip on Windows

- **WHEN** the window runs on Windows and the user points at the toolbar's Interrupt button
- **THEN** its tooltip says Ctrl+. and never ⌘ or Cmd

#### Scenario: A tooltip on macOS

- **WHEN** the window runs on macOS and the user points at the Docs toggle
- **THEN** its tooltip names the command, and a shortcut, if it has one, is in ⌘ symbols

### Requirement: The toolbar holds the session and the panels

The toolbar SHALL show, on the left, Run All, Interrupt and Restart beside the session status, and, on the right, toggles for the Variables, Docs and Log panels and a Settings button. Each SHALL be an icon button with a tooltip naming the command and its shortcut. A toggle SHALL show when its panel is open with a tint, not a solid fill. Interrupt SHALL be greyed out unless a cell is running. The Log toggle SHALL show the count of unread errors. The toolbar SHALL NOT show + Cell, + Markdown, Undo, Redo, Templates or the notebook's name; those are in the menu bar, the gaps between cells and the tab.

#### Scenario: The toolbar's controls

- **WHEN** a notebook window opens
- **THEN** the toolbar shows Run All, Interrupt, Restart, the status, the three panel toggles and Settings, and nothing else

#### Scenario: Interrupt while a cell runs

- **WHEN** a cell is running
- **THEN** Interrupt is enabled, and clicking it sends the same cancel the running cell's stop button sends

#### Scenario: A toggle shows its panel's state

- **WHEN** the user opens the docs panel from View > Docs
- **THEN** the toolbar's Docs toggle shows as on

### Requirement: The palette runs commands

The command palette SHALL list the commands whose labels match what the user types, above the functions and packages it lists now, each with its shortcut. Typing `>` first SHALL list only commands. Choosing a command SHALL run it and close the palette. Commands that are greyed out in the menu SHALL be left out.

#### Scenario: Finding Restart

- **WHEN** the user opens the palette and types "rest"
- **THEN** Restart is listed under Commands, and choosing it restarts the session

#### Scenario: Only commands

- **WHEN** the user types ">" in the palette
- **THEN** it lists every enabled command and no functions

### Requirement: Cells have a context menu

A right-click on a cell, outside the text of its editor, SHALL open a menu of commands for that cell: Run, Run and Stay, New Code Cell Above, New Code Cell Below, New Markdown Cell Below, Move Up, Move Down and Delete Cell, with their shortcuts. A right-click in the editor's text SHALL keep the system's text menu (cut, copy, paste). Escape or a click elsewhere SHALL close the menu.

#### Scenario: Deleting from the context menu

- **WHEN** the user right-clicks a cell's output and chooses Delete Cell
- **THEN** that cell is deleted, even if another cell was active, and Undo restores it

#### Scenario: Text keeps the system menu

- **WHEN** the user right-clicks selected text in a cell's editor
- **THEN** the system's menu with Cut, Copy and Paste opens, not Aximar's

### Requirement: A list of keyboard shortcuts

Help > Keyboard Shortcuts SHALL open a list of every command that has a shortcut, grouped by menu, with the platform's labels, followed by the cell editor's own keys (Shift+Enter, Mod+Enter, the arrow keys between cells, and Escape).

#### Scenario: The list matches the menus

- **WHEN** the user opens Help > Keyboard Shortcuts
- **THEN** each shortcut in the menu bar appears in the list once, beside its command's label

### Requirement: Icon buttons and panel headers look alike

Every button that shows only an icon SHALL use the app's icon set, with an accessible name and a tooltip. The variables, docs and log panels and the find bar SHALL share one header layout, with the panel's title or controls on the left and one close button, of one style, on the right.

#### Scenario: No text characters as icons

- **WHEN** the tests render the toolbar, the cell actions, the find bar and the panels
- **THEN** no button's only content is a text character such as ▲, ▼, ▶, ×, ← or →, and every icon-only button has an accessible name
