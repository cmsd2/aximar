## Purpose

How changes to notebooks and sessions reach the desktop window, whether the user, an MCP client or the app itself made them, so the window always shows the current state.

## ADDED Requirements

### Requirement: Every change reaches the window

Every change to a notebook's cells, statuses, outputs, trust, path or session status SHALL be delivered to the desktop window, whichever front end made it. No change SHALL be skipped because another delivery is in progress.

#### Scenario: Many changes at once

- **WHEN** an MCP client in connected mode adds 50 cells in quick succession while Maxima prints output from a running cell
- **THEN** the window SHALL show all 50 cells

#### Scenario: Session status

- **WHEN** an MCP client in connected mode restarts the session
- **THEN** the window SHALL show the session as Starting, then Ready (needs Maxima)

### Requirement: Changes arrive in the order they happened

Changes SHALL reach the window in the order they were made, and each SHALL carry the notebook state as it was right after that change.

#### Scenario: Status order

- **WHEN** an MCP client in connected mode runs a cell
- **THEN** the window SHALL receive Running before Success or Error for that cell, never the reverse (needs Maxima)

#### Scenario: Add then delete

- **WHEN** an MCP client in connected mode adds a cell and then deletes it
- **THEN** the window SHALL end with the cell absent

### Requirement: Notebook lifecycle reaches the window

Creating a notebook, closing it and changing the active notebook SHALL be delivered to the desktop window, whichever front end does it.

#### Scenario: Notebook created over MCP

- **WHEN** an MCP client in connected mode creates a notebook
- **THEN** the window SHALL open a tab for it

#### Scenario: Active notebook switched over MCP

- **WHEN** an MCP client in connected mode switches the active notebook
- **THEN** the window SHALL show that notebook's tab as active
