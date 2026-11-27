## Purpose

Aximar helps the user add its MCP server to the AI clients they have installed, and checks what those clients have configured without running their programs unasked.

## ADDED Requirements

### Requirement: Opening Settings runs no external program

Opening Settings, showing the AI assistants section and changing the address or the token SHALL NOT start any AI client's program. Aximar SHALL find whether a client is installed by looking up its command name on PATH, without running it.

#### Scenario: Opening AI assistants runs nothing

- **WHEN** the user opens Settings at the AI assistants section, with claude, codex and gemini on PATH
- **THEN** Aximar starts no child process, and each client row shows whether it is installed

#### Scenario: A client not on PATH

- **WHEN** no codex command is on PATH
- **THEN** the Codex row says the codex command wasn't found and its Configure button is disabled

### Requirement: Codex and Gemini status comes from their files

Aximar SHALL decide whether Codex lists its server by reading `~/.codex/config.toml` for an `aximar` entry under `mcp_servers`, and whether Gemini CLI lists it by reading `~/.gemini/settings.json` for an `aximar` entry under `mcpServers`. A missing file SHALL count as not configured. A file that can't be parsed SHALL show an error naming the file, and Aximar SHALL NOT write to it.

#### Scenario: Codex configured

- **WHEN** `~/.codex/config.toml` has a table `[mcp_servers.aximar]`
- **THEN** the Codex row shows Configured

#### Scenario: A broken Gemini file

- **WHEN** `~/.gemini/settings.json` holds text that isn't JSON
- **THEN** the Gemini CLI row shows an error naming the file, and Configure leaves the file unchanged

### Requirement: Claude Code is checked only on request

Aximar SHALL run `claude mcp get aximar` only when the user presses the Claude Code row's Check button, and once after a Configure for Claude Code finishes. Until then the row SHALL show that its status hasn't been checked. A check SHALL time out after 10 seconds and show that it timed out.

#### Scenario: Unchecked at first

- **WHEN** the user opens the AI assistants section with claude on PATH
- **THEN** the Claude Code row shows Not checked and a Check button, and Aximar has not run claude

#### Scenario: Check runs the CLI

- **WHEN** the user presses Check and `claude mcp get aximar` exits with success
- **THEN** the row shows Configured

#### Scenario: Configure checks afterwards

- **WHEN** the user presses Configure for Claude Code and `claude mcp add` succeeds
- **THEN** Aximar runs `claude mcp get aximar` once and the row shows its result

### Requirement: The server status box reports the MCP server's state

The AI assistants section SHALL show, in a status box, whether the MCP server is off, listening (with its address), or failed to start (with the reason, such as the address being in use).

#### Scenario: Address in use

- **WHEN** another program holds the configured port at startup
- **THEN** the status box says the server couldn't listen on that address and gives the operating system's reason

#### Scenario: Listening

- **WHEN** the MCP server is on and listening on 127.0.0.1:19542
- **THEN** the status box says it is listening on 127.0.0.1:19542
