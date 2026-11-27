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

**Tool sets.** By default the standalone binary offers the session-oriented tools ("simple mode"): `evaluate_expression`, session and log tools, and documentation search. `--notebook` offers the full notebook tool set described under [Available tools](#available-tools-24). The connected mode always offers the full set.

In simple mode, **evaluate_expression(expression, session_id?, cell_id?, cells_above?)** can run the expression as a cell of a notebook the client keeps itself, such as the VS Code extension's. Pass the cell's `cell_id`, which stays the same while the notebook is open, and `cells_above`, the IDs of the code cells above it in order. The server then numbers the runs (`execution_count` in the result, the gutter number), and resolves `%oN` as that notebook's `Out[N]` and a bare `%` as the output of the nearest cell above that has one in this session, as the Aximar window does. Without `cell_id`, the expression keeps Maxima's own labels, as clients that resolve labels themselves expect. Clients detect support from the tool's input schema, which lists `cell_id` and `cells_above` (tools 0.6.0 and later).

### Connected mode (embedded in Tauri app)

When the Aximar desktop app is running, it can host an MCP streamable HTTP server that shares the GUI's Maxima session and notebook state, so MCP-triggered changes appear live in the app.

A session that MCP starts uses the Maxima backend, Maxima path and evaluation timeout from Settings as they are when the session starts. The environment variables above apply only to standalone mode.

**Enable the MCP server** in the AI assistants section of Settings by toggling the "MCP server" checkbox. A line under it says whether the server is listening, or why it couldn't listen, such as another program holding the port. The listen address, under Advanced, defaults to `127.0.0.1:19542` and can be changed without restarting the app; the server restarts by itself. When the server can't listen, its log entry links to the AI assistants section.

**Opening Settings runs no AI client's program.** Aximar finds each client's command on PATH, and in the folders their installers use (`~/.local/bin`, `~/.claude/local`, `~/.npm-global/bin`, `/opt/homebrew/bin`, `/usr/local/bin`), since an app opened from the macOS Finder gets a short PATH. It reads `~/.codex/config.toml` and `~/.gemini/settings.json` to see whether Codex and Gemini CLI list Aximar, and leaves a file it can't parse alone. Claude Code's CLI is the only supported way to read its configuration, so Aximar runs `claude mcp get aximar` only when you press **Check**, and once after **Configure**, with a 10-second limit.

#### Authentication

The connected-mode server requires a **bearer token** on every HTTP request. A random 256-bit token is generated on first launch and stored in the app config. You can view, copy, or regenerate the token in Settings, under Advanced in the AI assistants section.

#### Configuring Claude Code

The easiest way is the **Configure** button in Settings (next to "Claude Code"). It runs the `claude` Aximar found to register the MCP server with the correct URL and bearer token, then checks the result. Click **Reconfigure** after regenerating the token or changing the listen address.

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

In both modes `load` of a Maxima share package (`load("distrib")`) or of a package installed with mxpm (`load("numerics")`, or a file it ships such as `load("numerics-optimize")`) is not a dangerous call.

The check matches names in the cell text, so it warns about notebooks that use these functions openly but cannot stop one written to get past it (see `docs/security.md`).

`aximar-mcp run <notebook>` does not apply the check: running the command is the decision to run the notebook, so every cell runs. `run --allow-dangerous` is still accepted, with a warning that it is no longer needed.

In `run_all_cells`, a blocked cell stops the run with status `needs_trust`.

## Available tools (24)

### Documentation

- **search_functions(query, language?)** -- Search the Maxima function catalog by name or description. The catalog's ranked results come first, with a `score`; functions that installed packages match by name follow without one, marked `undocumented: true` when there are no docs for them. The query matches every installed language; each result's `description` is in `language` where there is a translation, and its `language` field names which.
- **get_function_docs(name, language?)** -- Get full documentation for a Maxima function. For a function that only a package provides, it names every such package and how to load it. A misspelt name gets suggestions from the catalog and the packages. With `language` (a BCP 47 tag such as `es`), the result is in that language where there is a translation: `language` and `description_language` name the language of `full_docs` and `description`, `fallback_from` marks a function with nothing in that language, and `stale` with `english_signatures` marks a translation whose signatures differ from the English. Without `language`, both tools answer in English.
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

`run_cell` and `evaluate_expression` return the run's output: `text_output` (print text), `latex` and `text_plain` (the final value as LaTeX, and in Maxima's one-line form, as `string()` writes it), `plot_svg`, `plot_data` (Plotly JSON), `image_png` (base64), `error` and `is_error`. Notebook files keep `text_plain` as the result's `text/plain` (see `docs/notebook-format.md`).

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

  Files are read and written as `docs/notebook-format.md` describes, by the same code as the desktop window and the VS Code extension: nbformat 4.5, every cell's `id` kept (a missing, invalid or repeated one is replaced), notebook and cell metadata and raw cells kept, and `error` outputs read back as errors. Outputs read from a file are stale: opening a notebook numbers runs from 1 again, and `%oN` for a stale output reports that it has no value in this session.

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
