## ADDED Requirements

### Requirement: Every front end reads and writes files the same way

Opening and saving SHALL go through one serializer in aximar-core, whether the desktop window, MCP or a batch run does it, so a notebook opened or saved by one front end is the notebook the others see.

#### Scenario: Saved by the window, opened over MCP

- **WHEN** the window saves a notebook with an error output and cell metadata, and an MCP client opens the file
- **THEN** `get_cell` SHALL report the same cell IDs, error and input the window saved

#### Scenario: Opening records the path in one step

- **WHEN** the window opens `work/calc.macnb`
- **THEN** the backend SHALL read the file, load the cells and record the path in one command, and the tab SHALL be titled `calc.macnb`
