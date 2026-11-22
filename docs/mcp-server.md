# MCP Server

Aximar exposes its Maxima CAS capabilities via the [Model Context Protocol](https://modelcontextprotocol.io/), allowing AI assistants (Claude Code, etc.) to search function documentation, manage notebook cells, run Maxima expressions, and inspect session state.

## Two modes of operation

### Headless mode (standalone binary)

The `aximar-mcp` binary runs independently with its own Maxima session, using stdio transport. Suitable for scripted/automated use with Claude Code or other MCP clients.

**Setup in Claude Code:**

```json
{
  "mcpServers": {
    "aximar": {
      "command": "/path/to/aximar-mcp"
    }
  }
}
```

**Environment variables:**

| Variable | Default | Description |
|---|---|---|
| `AXIMAR_BACKEND` | `local` | `local`, `docker`, or `wsl` |
| `AXIMAR_MAXIMA_PATH` | (system default) | Path to the Maxima binary |
| `AXIMAR_EVAL_TIMEOUT` | `30` | Evaluation timeout in seconds |
| `AXIMAR_DOCKER_IMAGE` | (empty) | Docker image name (when backend=docker) |
| `AXIMAR_WSL_DISTRO` | (empty) | WSL distribution name (when backend=wsl) |
| `AXIMAR_CONTAINER_ENGINE` | `docker` | `docker` or `podman` |

### Connected mode (embedded in Tauri app)

When the Aximar desktop app is running, it can host an MCP streamable HTTP server that shares the GUI's Maxima session and notebook state, so MCP-triggered changes appear live in the app.

A session that MCP starts uses the Maxima backend, Maxima path and evaluation timeout from Settings as they are when the session starts. The environment variables above apply only to standalone mode.

**Enable the MCP server** in Settings by toggling the "MCP server" checkbox. The listen address defaults to `127.0.0.1:19542` and can be changed without restarting the app — the server restarts automatically.

#### Authentication

The connected-mode server requires a **bearer token** on every HTTP request. A random 256-bit token is generated on first launch and stored in the app config. You can view, copy, or regenerate the token in Settings under "MCP token".

#### Configuring Claude Code

The easiest way is the **Configure** button in Settings (next to "Claude Code"). It runs the necessary CLI commands to register the MCP server with the correct URL and bearer token. Click **Reconfigure** after regenerating the token or changing the listen address.

To configure manually via the CLI:

```bash
claude mcp add --transport http \
  --header "Authorization: Bearer <token>" \
  -- aximar http://localhost:19542/mcp
```

Replace `<token>` with the value shown in Settings. Alternatively, add it to `.mcp.json`:

```json
{
  "mcpServers": {
    "aximar": {
      "type": "http",
      "url": "http://localhost:19542/mcp",
      "headers": {
        "Authorization": "Bearer <token>"
      }
    }
  }
}
```

#### Configuring Codex

Use the **Configure** button in Settings (next to "Codex") to automatically register the MCP server. This writes the server entry to `~/.codex/config.toml` with the correct URL and bearer token.

To configure manually, add the following to `~/.codex/config.toml`:

```toml
[mcp_servers.aximar]
url = "http://localhost:19542/mcp"
http_headers = { "Authorization" = "Bearer <token>" }
```

Replace `<token>` with the value shown in Settings.

#### Configuring Gemini CLI

Use the **Configure** button in Settings (next to "Gemini CLI") to automatically register the MCP server. This writes the server entry to `~/.gemini/settings.json` with the correct URL and bearer token.

To configure manually, add the following to `~/.gemini/settings.json`:

```json
{
  "mcpServers": {
    "aximar": {
      "httpUrl": "http://localhost:19542/mcp",
      "headers": {
        "Authorization": "Bearer <token>"
      }
    }
  }
}
```

Replace `<token>` with the value shown in Settings.

## Dangerous functions

Code that calls `system`, `batch`, `writefile` and similar functions, or loads a file that isn't a known package, can read or change files on your machine. `run_cell`, `run_all_cells` and `evaluate_expression` all apply one check before running such code (`safety::check_trust` in aximar-core), the same check the desktop window uses:

- **Connected mode:** the notebook's trust flag decides. New notebooks are trusted; notebooks opened from a file are not until the user trusts them. A blocked cell returns `needs_notebook_trust` with its `dangerous_functions`, and the window shows its trust banner so the user can trust the notebook there. A blocked expression returns an error.
- **Standalone mode:** dangerous calls are blocked unless the server was started with `--allow-dangerous`, whatever the notebook's trust flag.

In `run_all_cells`, a blocked cell stops the run with status `needs_trust`.

## Available tools (24)

### Documentation

- **search_functions(query)** -- Search the Maxima function catalog by name or description. The catalog's ranked results come first, with a `score`; functions that installed packages match by name follow without one, marked `undocumented: true` when there are no docs for them.
- **get_function_docs(name)** -- Get full documentation for a Maxima function. For a function that only a package provides, it names every such package and how to load it. A misspelt name gets suggestions from the catalog and the packages.
- **complete_function(prefix)** -- Autocomplete a function name prefix. Each name appears once, naming every package that provides it.

These answer from the same core API as the desktop window (`catalog::queries`), so both find the same functions.

### Packages

- **search_packages(query)** -- Search available Maxima packages by name or description.
- **list_packages()** -- List all available Maxima packages that can be loaded with `load("name")`.
- **get_package(name)** -- Get details of a specific package, including its functions.

### Cell management

- **list_cells()** -- List all cells with IDs, types, status, and content preview.
- **get_cell(cell_id)** -- Get full details of a specific cell.
- **add_cell(cell_type?, input?, after_cell_id?)** -- Add a new cell.
- **update_cell(cell_id, input?, cell_type?)** -- Update a cell's content or type.
- **delete_cell(cell_id)** -- Delete a cell (cannot delete the last cell).
- **move_cell(cell_id, direction)** -- Move a cell up or down.

### Execution

- **run_cell(cell_id)** -- Execute a notebook cell (auto-starts session).
- **run_all_cells()** -- Execute all code cells in order. Stops at the first
  cell that errors, fails, or needs notebook trust. Each entry in `results` has
  a `status`: `ok`, `empty`, `error` (with `error` text), `needs_trust` (with
  `dangerous_functions`), `failed` (with `error`), or `not_run`. Top-level
  `success` is true only when every code cell ran cleanly; `stopped_at` and
  `stop_reason` name the cell that stopped the run. `stop_reason` is `error`,
  `needs_trust`, `failed`, or `cancelled` when the user stops the run from the
  desktop window (then `stopped_at` is the first cell not run). The desktop
  window's Run All follows the same rule.
- **evaluate_expression(expression)** -- Quick evaluation without creating a cell. The expression is prepared like a cell: Unicode is translated (see `docs/unicode-input.md`) and `%oN` / `%iN` refer to the cells' gutter numbers. A bare `%` keeps Maxima's meaning, the last result.

### Session

- **get_session_status()** -- Get current status (Starting/Ready/Busy/Stopped/Error).
- **restart_session()** -- Kill and restart the Maxima process.
- **list_variables()** -- List user-defined variables.
- **kill_variable(name)** -- Remove a variable from the session.

### Logs

- **get_cell_output_log(cell_id)** -- Raw Maxima I/O for a cell.
- **get_server_log(stream?, limit?)** -- Server-wide output log.

### Notebook I/O

- **save_notebook(path)** -- Save as Jupyter .ipynb, and record `path` as the notebook's file.
- **open_notebook(path)** -- Open a .ipynb file, and record `path` as the notebook's file.

  The recorded path shows in `list_notebooks` and, in connected mode, as the window's tab title and save path. Maxima runs in the file's folder, so `load("helpers.mac")` finds a file next to the notebook. A session already running keeps its folder until `restart_session`.
- **list_templates()** -- List available notebook templates.
- **load_template(template_id)** -- Load a template into the notebook. Clears the recorded path, since the notebook no longer matches the file.

## Architecture

All notebook mutations (add/delete/move cell, toggle type, update input, execute, undo/redo) flow through a **command-effect system** in the Rust backend. Both the GUI frontend and MCP tools dispatch `NotebookCommand` variants to `Notebook::apply()`, which returns a `CommandEffect` describing what changed. The backend is the single source of truth; the frontend receives state updates via `notebook-state-changed` Tauri events.

Operations both front ends offer live in aximar-core, and the Tauri commands and MCP tools only translate parameters and results: running a cell and reporting its statuses as they happen (`evaluation::evaluate_cell`), running all cells (`evaluation::run_all`), the trust check (`safety::check_trust`), input preparation (`evaluation::prepare_input`), notebook paths (`NotebookRegistry::set_path`) and catalog queries (`catalog::queries`). `openspec/specs/` holds the requirements they meet.

```
crates/aximar-core/     Shared library
  src/notebook.rs       Notebook state + command application + undo/redo
  src/commands.rs       NotebookCommand and CommandEffect enums
  src/capture.rs        Per-cell output capture (CaptureOutputSink)
  src/log.rs            Server-wide output ring buffer (ServerLog)
  src/session.rs        SessionManager (Maxima process lifecycle)
  src/session_ops.rs    Session start, with SessionSettings
  src/evaluation.rs     evaluate_cell, run_all, prepare_input, check_cell_trust
  src/safety.rs         Dangerous-call detection and the trust check
  src/maxima/unicode.rs Unicode in Maxima code (docs/unicode-input.md)
  src/catalog/queries.rs Function search, docs and completion

crates/aximar-mcp/      MCP server (lib + binary)
  src/lib.rs            Re-exports notebook, commands, capture, log from aximar-core
  src/main.rs           Headless binary entry point (stdio transport)
  src/server.rs         AximarMcpServer and its tools
  src/config.rs         Environment variable configuration

src-tauri/src/
  commands/notebook.rs  Tauri notebook commands (nb_add_cell, nb_run_cell, etc.)
  mcp/startup.rs        HTTP server startup (StreamableHttpService on localhost)
  gui_events.rs         One ordered channel from MCP changes to the window
  mcp/sync.rs           Event payload helpers (SyncCell, notebook_state_payload)
```

In connected mode, the MCP server shares `SessionManager`, `Catalog`, `Docs`, `Notebook`, and output sinks with the Tauri app via `Arc`. Changes it makes reach the window in order through one channel, drained by a single task (`gui_events.rs`); see `docs/core-tauri-data-transfer.md`. A `MultiOutputSink` broadcasts Maxima I/O to both the Tauri frontend (for the GUI log) and the `CaptureOutputSink` (for MCP cell output capture).
