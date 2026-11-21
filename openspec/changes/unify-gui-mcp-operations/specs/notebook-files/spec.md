## Purpose

What a notebook records when it is opened from or saved to a file, from the desktop window or an MCP client, and what each front end then reports about it.

## ADDED Requirements

### Requirement: Opening or saving records the file path

Opening a notebook from a file or saving it to a file SHALL record that file as the notebook's path, whichever front end does it. Saving to a new path SHALL replace the recorded path.

#### Scenario: Opened over MCP

- **WHEN** an MCP client opens `examples/demo.macnb` into a notebook
- **THEN** `list_notebooks` SHALL report that notebook with path `examples/demo.macnb` and title `demo.macnb`

#### Scenario: Saved from the window

- **WHEN** the user saves a new notebook from the window as `work/calc.macnb`
- **THEN** `list_notebooks` over MCP SHALL report that notebook with path `work/calc.macnb`

#### Scenario: Save as a new file

- **WHEN** a notebook opened from `a.macnb` is saved as `b.macnb`
- **THEN** its recorded path SHALL be `b.macnb`

### Requirement: The window shows the recorded path

The desktop window SHALL show a notebook's file name as its tab title and SHALL save to the recorded path, whichever front end recorded it. Only opening, saving or loading a template SHALL change the path: running or editing cells SHALL NOT clear it. Loading a template into a notebook SHALL clear its path, because the notebook no longer matches the file.

#### Scenario: Opened over MCP, saved from the window

- **WHEN** an MCP client in connected mode opens `demo.macnb`, and the user then presses Save in the window
- **THEN** the tab SHALL be titled `demo.macnb`, and the window SHALL write to `demo.macnb` without asking for a file name

#### Scenario: Running cells keeps the path

- **WHEN** a notebook opened from `demo.macnb` runs all its cells
- **THEN** the tab title SHALL stay `demo.macnb`

#### Scenario: Template replaces a notebook with a path

- **WHEN** a notebook opened from `demo.macnb` has its cells replaced by loading a template over MCP
- **THEN** `list_notebooks` SHALL report no path for it, the tab SHALL be titled `Untitled`, and Save in the window SHALL ask for a file name

### Requirement: Maxima runs in the notebook's folder

A session started for a notebook with a recorded path SHALL use the folder containing that file as Maxima's working directory, so relative `load` calls find files next to the notebook. A path recorded while a session is running SHALL apply from the session's next start.

#### Scenario: Relative load next to the notebook

- **WHEN** `work/calc.macnb` is opened from either front end, `work/helpers.mac` defines `h(x) := x + 1`, and the notebook (trusted) runs `load("helpers.mac")$ h(1);`
- **THEN** the result SHALL be `2` (needs Maxima)
