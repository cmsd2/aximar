# cell-execution Specification

## Purpose
How cells and expressions run, from the desktop window or an MCP client: which Maxima a session uses, the trust check before dangerous calls, the statuses a run reports, and when running all cells stops.

## Requirements

### Requirement: Both front ends run cells the same way

Running a cell SHALL behave the same whether the desktop window or an MCP client starts it: the same input preparation, the same trust check, the same statuses and the same result.

#### Scenario: Same result from either front end

- **WHEN** the same cell containing `α: 2; α^2;` runs once from the window and once over MCP in connected mode
- **THEN** both runs SHALL give `4` and leave the cell in the Success status (needs Maxima)

#### Scenario: Unicode in an expression

- **WHEN** an expression `α + 1` is evaluated over MCP and through the desktop app's expression command
- **THEN** both SHALL send `alpha + 1` to Maxima

### Requirement: Sessions use the configured Maxima

A session started for a notebook SHALL use the backend, Maxima path and evaluation timeout configured at the moment it starts, whichever front end starts it.

#### Scenario: MCP starts the session in the desktop app

- **WHEN** the app's settings name a Maxima path that does not exist, and an MCP client in connected mode runs a cell before the window has started a session
- **THEN** the session SHALL fail to start with an error naming that path

#### Scenario: Settings changed after startup

- **WHEN** the timeout setting changes from 30 to 120 seconds, then a session starts over MCP
- **THEN** that session SHALL use 120 seconds

### Requirement: Every run reports its statuses as they happen

A cell run SHALL report Running when evaluation starts and Success or Error when it ends, including a timeout, a cancellation or a failure to start Maxima. The desktop window SHALL show these statuses for runs started from either front end.

#### Scenario: MCP run seen in the window

- **WHEN** an MCP client in connected mode runs a cell containing `for i:1 thru 3000000 do x:i;`
- **THEN** the window SHALL show the cell as Running before the run ends, then Success (needs Maxima)

#### Scenario: Failed run seen in the window

- **WHEN** an MCP client in connected mode runs a cell containing `1/0;`
- **THEN** the window SHALL show the cell with the Error status and the error output (needs Maxima)

### Requirement: One trust check before dangerous calls

Running a cell, running all cells and evaluating an expression SHALL apply the same check for dangerous calls (such as `system` or loading an unknown file). In the desktop app and in MCP connected to it, a dangerous call SHALL be blocked only while the notebook is untrusted. In standalone MCP, it SHALL be blocked unless the server was started with `--allow-dangerous`. Batch runs (`aximar-mcp run`) SHALL NOT apply the check, since running the command is the decision to run the notebook.

#### Scenario: Expression in a trusted notebook over connected MCP

- **WHEN** the notebook is trusted and an MCP client in connected mode evaluates an expression that calls `system`
- **THEN** the expression SHALL run

#### Scenario: Cell in an untrusted notebook

- **WHEN** the notebook is untrusted and a cell calls `load("helpers.mac")` with no such package installed, from either front end
- **THEN** the cell SHALL NOT run, and the result SHALL list `load` as needing trust

#### Scenario: Share package written in Lisp

- **WHEN** the notebook is untrusted and a cell calls `load("orthopoly")`, `load("draw")` or `load("ctensor")`, packages shipped in Maxima's share directory
- **THEN** the cell SHALL run

#### Scenario: Batch run

- **WHEN** `aximar-mcp run` runs a notebook whose cell calls `load("helpers.mac")`, without `--allow-dangerous`
- **THEN** the cell SHALL run (needs Maxima)

#### Scenario: Standalone MCP

- **WHEN** standalone MCP without `--allow-dangerous` runs a cell that calls `system`, in a trusted notebook
- **THEN** the cell SHALL NOT run

#### Scenario: Package installed with mxpm

- **WHEN** a batch run without `--allow-dangerous` reaches a cell that calls `load("numerics")` or `load("numerics-optimize")`, and mxpm installed `numerics` (which ships `numerics-optimize.mac`) in the Maxima user directory
- **THEN** the cell SHALL run

#### Scenario: Path into a package folder

- **WHEN** an untrusted notebook's cell calls `load("numerics/numerics")` or `load("../numerics")`
- **THEN** the cell SHALL NOT run, and the result SHALL list `load` as needing trust

### Requirement: A blocked cell asks for trust in the window

When a cell is blocked by the trust check in the desktop app or connected MCP, the window SHALL show its trust banner listing the blocked functions, whichever front end ran the cell.

#### Scenario: Blocked over MCP

- **WHEN** an MCP client in connected mode runs a cell that calls `load("helpers.mac")` in an untrusted notebook
- **THEN** the window SHALL show the trust banner listing `load`

### Requirement: Running all cells stops at the first cell that doesn't succeed

Running all cells SHALL run code cells in notebook order and stop at the first cell that ends in an error, is blocked by the trust check, or fails to run. It SHALL report each cell's outcome, marking cells after the stop as not run. Empty code cells SHALL be skipped. The window's Run All and MCP's `run_all_cells` SHALL follow the same rule.

#### Scenario: Error in the middle

- **WHEN** a notebook's code cells are `a: 1;`, `1/0;` and `b: 2;`, and all cells run
- **THEN** the run SHALL stop at the second cell, report it as an error and the third as not run, and `b` SHALL stay unbound (needs Maxima)

#### Scenario: Trust block in the middle

- **WHEN** an untrusted notebook's code cells are `a: 1;`, `load("helpers.mac");` and `b: 2;`, and all cells run
- **THEN** the run SHALL stop at the second cell and report it as needing trust (needs Maxima)

#### Scenario: Pending edits are included

- **WHEN** the user types into a cell and clicks Run All within 300 ms
- **THEN** that cell SHALL run with the text just typed

#### Scenario: Stopping a run

- **WHEN** all cells are running and the user stops the running cell
- **THEN** no later cell SHALL run (needs Maxima)
