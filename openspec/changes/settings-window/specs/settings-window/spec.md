## Purpose

The Settings window groups Aximar's settings into sections a user can find, explains each one, and opens at the right place when another part of the app sends the user there.

## ADDED Requirements

### Requirement: Settings are grouped into sections

The Settings window SHALL show a sidebar, headed by the window's title, listing its sections with an icon each, and SHALL show one section's settings at a time, under the section's name. The Translation section SHALL appear greyed in the sidebar. The sections and their settings, in this order, SHALL be:

- General: language, help language, theme, variables panel open at start.
- Notebook: font size, cell style, argument help, suggestions under results, Markdown font, Markdown indent.
- Maxima: backend, the fields for the chosen backend, a Maxima status line, eval timeout.
- Printing: print font size, print margins.
- AI assistants: MCP server switch, MCP server status line, one row per AI client, and an Advanced disclosure holding the listen address and the token.
- Translation: translation preview, show sources.

The Translation section SHALL appear in every build. Every setting the window offers before this change SHALL appear in exactly one section.

#### Scenario: Every build lists six sections

- **WHEN** any build opens Settings
- **THEN** the sidebar lists General, Notebook, Maxima, Printing, AI assistants and Translation, and choosing Translation shows the translation preview and show-sources rows

#### Scenario: Choosing a section shows its settings

- **WHEN** the user chooses Printing in the sidebar
- **THEN** the page shows the print font size and the four margin fields, and no other settings

#### Scenario: No setting is lost

- **WHEN** the stories render each section in turn
- **THEN** every config field the old window edited appears on exactly one section's page

### Requirement: The Maxima section shows only the chosen backend's fields

The Maxima section SHALL show the Maxima path field only for the Local backend, the container engine and image fields only for Docker, and the distro menu only for WSL. Switching the backend SHALL keep the other backends' values in the config file.

#### Scenario: Docker shows its fields

- **WHEN** the user chooses the Docker backend
- **THEN** the section shows the container engine and image fields and hides the Maxima path field and the distro menu

#### Scenario: Switching back keeps the path

- **WHEN** the user sets a Maxima path, switches to Docker and then back to Local
- **THEN** the Maxima path field shows the path the user set

### Requirement: The Maxima section says which Maxima Aximar uses

For the Local backend the Maxima section SHALL show a status box naming the Maxima program Aximar would start, its version, and where its path came from (the Maxima path setting, the AXIMAR_MAXIMA_PATH environment variable, a standard install location, or PATH), or saying that Aximar found none. For WSL it SHALL say whether the chosen distro has a maxima program. The status SHALL update when the user changes the Maxima path, the backend or the distro. Finding the program SHALL NOT start a Maxima session; to read the version Aximar SHALL run the program found once with `--version`, giving up after 5 seconds, and SHALL NOT run a path that names no file.

#### Scenario: The setting names the program

- **WHEN** the Maxima path setting holds the path of a Maxima 5.50.0 program
- **THEN** the status box reads "Found Maxima 5.50.0 at" that path, and says it comes from the setting

#### Scenario: Nothing found

- **WHEN** the Maxima path is empty, AXIMAR_MAXIMA_PATH is unset and no standard location holds maxima
- **THEN** the status box says Aximar found no Maxima and points to the Maxima path field

#### Scenario: A path that doesn't exist

- **WHEN** the user types a Maxima path that names no file
- **THEN** the status box says the file doesn't exist, and Aximar runs nothing

### Requirement: Each setting has a one-line hint

Each setting SHALL show a hint of one line under its label saying what the setting changes. Hints SHALL be translatable text with a translator note.

#### Scenario: Argument help explains itself

- **WHEN** the Notebook section is shown
- **THEN** the argument help setting shows a hint saying how each choice shows a function's arguments while typing

### Requirement: Suggestion chips can be turned off

The Notebook section SHALL offer a checkbox, on by default, that shows the suggestion chips under a cell's result. Turned off, no cell SHALL show chips or ask the backend for them, so a teacher can have students write each step themselves (cmsd2/aximar#44).

#### Scenario: A teacher turns the chips off

- **WHEN** the user clears "Suggestions under results"
- **THEN** results show no suggestion chips, at once and after a restart

#### Scenario: A config without the setting shows chips

- **WHEN** Aximar reads a config file written before the setting existed
- **THEN** suggestion chips show under results

### Requirement: Settings apply at once

A change to a setting SHALL take effect and SHALL be saved to the config file as soon as the user makes it, without a Save or OK button. Text fields SHALL save when they lose focus or the user presses Enter. The page SHALL end with a note saying changes apply as they're made, and a close button SHALL sit beside the section's name.

#### Scenario: A theme change shows at once

- **WHEN** the user chooses the Dark theme
- **THEN** the window turns dark while Settings is still open, and the config file holds the dark theme

#### Scenario: The listen address saves on blur

- **WHEN** the user types a new listen address and moves focus out of the field
- **THEN** the config file holds the new address, and no keystroke before that wrote it

### Requirement: Other parts of the app open a given section

The app SHALL let any part of the window open Settings at a named section. The Maxima Not Found dialog SHALL offer an Open Settings button that opens the Maxima section. When the MCP server fails to listen on its address, the log entry that reports it SHALL carry a link that opens the AI assistants section.

#### Scenario: Maxima Not Found opens Maxima

- **WHEN** Aximar can't find Maxima and the user presses Open Settings in the dialog
- **THEN** Settings opens at the Maxima section

#### Scenario: A bind failure links to AI assistants

- **WHEN** the MCP server can't listen because another program holds the port, and the user follows the link in the log entry
- **THEN** Settings opens at the AI assistants section, whose status box gives the reason

### Requirement: Settings reopens where the user left it

Opening Settings without naming a section SHALL show the section the user last chose, across restarts of the app. Opening it at a named section SHALL show that section and SHALL make it the remembered one. If the remembered section isn't available in this build, the window SHALL show General.

#### Scenario: The last section comes back

- **WHEN** the user chooses Printing, closes Settings, quits Aximar, starts it again and opens Settings from the toolbar
- **THEN** Settings opens at Printing

#### Scenario: Translation in a release build

- **WHEN** the remembered section is Translation and a release build opens Settings
- **THEN** Settings opens at General

### Requirement: Settings works from the keyboard

Cmd+, on macOS and Ctrl+, elsewhere SHALL open Settings, through a Settings menu item that shows the shortcut. Esc SHALL close Settings. With focus in the sidebar, the Up and Down arrow keys SHALL move to the previous and next section and show it, Home and End SHALL move to the first and last, and Tab SHALL move focus to the section's first field. Esc inside an open select menu SHALL close the menu, not the window.

#### Scenario: The shortcut opens Settings

- **WHEN** the user presses Cmd+, on macOS with a notebook open
- **THEN** Settings opens at the remembered section

#### Scenario: Arrows move through the sidebar

- **WHEN** focus is on General in the sidebar and the user presses Down twice
- **THEN** the Maxima section is shown and its sidebar entry has focus

#### Scenario: Esc closes

- **WHEN** Settings is open and no menu inside it is open, and the user presses Esc
- **THEN** Settings closes

### Requirement: The window's text is translatable

Every string the Settings window shows SHALL come from the catalog with a translator note, except product names (Docker, Podman, WSL, Claude Code, Codex, Gemini CLI, Computer Modern) and values the user typed. Each sidebar label SHALL carry a `max-length:16` note. The catalog test, which already fails on a translation over its limit, SHALL cover these entries.

#### Scenario: Sidebar labels carry a limit

- **WHEN** the template is extracted
- **THEN** each sidebar label's entry in `locales/messages.pot` has a note containing `max-length:16`

#### Scenario: A long sidebar translation fails the catalog test

- **WHEN** `es.po` translates the AI assistants sidebar label with more than 16 characters
- **THEN** the catalog test fails and names the entry and the limit

#### Scenario: The pseudo-locale covers the window

- **WHEN** the Settings stories render in the pseudo-locale
- **THEN** every label, hint, button and status line shows pseudo-locale text, except the product names and typed values
