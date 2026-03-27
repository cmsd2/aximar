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

## Available tools (24)

### Documentation

- **search_functions(query)** -- Search the Maxima function catalog by name or description.
- **get_function_docs(name)** -- Get full documentation for a Maxima function.
- **complete_function(prefix)** -- Autocomplete a function name prefix.

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
- **run_all_cells()** -- Execute all code cells in order.
- **evaluate_expression(expression)** -- Quick evaluation without creating a cell.

### Session

- **get_session_status()** -- Get current status (Starting/Ready/Busy/Stopped/Error).
- **restart_session()** -- Kill and restart the Maxima process.
- **list_variables()** -- List user-defined variables.
- **kill_variable(name)** -- Remove a variable from the session.

### Logs

- **get_cell_output_log(cell_id)** -- Raw Maxima I/O for a cell.
- **get_server_log(stream?, limit?)** -- Server-wide output log.

### Notebook I/O

- **save_notebook(path)** -- Save as Jupyter .ipynb.
- **open_notebook(path)** -- Open a .ipynb file.
- **list_templates()** -- List available notebook templates.
- **load_template(template_id)** -- Load a template into the notebook.

## Architecture

All notebook mutations (add/delete/move cell, toggle type, update input, execute, undo/redo) flow through a **command-effect system** in the Rust backend. Both the GUI frontend and MCP tools dispatch `NotebookCommand` variants to `Notebook::apply()`, which returns a `CommandEffect` describing what changed. The backend is the single source of truth; the frontend receives state updates via `notebook-state-changed` Tauri events.

```
crates/aximar-core/     Shared library
  src/notebook.rs       Notebook state + command application + undo/redo
  src/commands.rs       NotebookCommand and CommandEffect enums
  src/capture.rs        Per-cell output capture (CaptureOutputSink)
  src/log.rs            Server-wide output ring buffer (ServerLog)
  src/session.rs        SessionManager (Maxima process lifecycle)

crates/aximar-mcp/      MCP server (lib + binary)
  src/lib.rs            Re-exports notebook, commands, capture, log from aximar-core
  src/main.rs           Headless binary entry point (stdio transport)
  src/server.rs         AximarMcpServer with 21 tool implementations
  src/config.rs         Environment variable configuration

src-tauri/src/
  commands/notebook.rs  Tauri notebook commands (nb_add_cell, nb_run_cell, etc.)
  mcp/startup.rs        HTTP server startup (StreamableHttpService on localhost)
  mcp/sync.rs           Event payload helpers (SyncCell, notebook_state_payload)
```

In connected mode, the MCP server shares `SessionManager`, `Catalog`, `Docs`, `Notebook`, and output sinks with the Tauri app via `Arc`. A `MultiOutputSink` broadcasts Maxima I/O to both the Tauri frontend (for the GUI log) and the `CaptureOutputSink` (for MCP cell output capture).
