# Backend Code Quality

Audit performed 2026-03-24 against `master` at commit `8d6f1c3`.

## Critical

### 1. server.rs is a god file (1,057 lines)

`crates/aximar-mcp/src/server.rs` implements all 21 MCP tools as methods on a single struct. Mixes documentation search, cell management, execution, session control, notebook I/O, logging, and template loading.

**Specific issues:**
- Two constructors (`new`, `new_connected`) with 18-line field assignment duplication
- Three documentation tools with near-identical JSON serialisation patterns
- `run_cell` is a 135-line monolith with 4+ nested lock acquisitions across notebook and session mutexes
- `save_notebook`, `open_notebook`, `load_template` share identical lock-apply-notify-drop boilerplate

**Suggested refactoring:**
- Split into handler modules: `docs_tools.rs`, `cell_tools.rs`, `execution_tools.rs`, `session_tools.rs`, `notebook_io_tools.rs`
- Extract `run_cell` into smaller functions: `prepare_cell_input()`, `set_cell_running()`, `evaluate_and_capture()`, `apply_output()`
- Create a helper `apply_notebook_command()` to centralise the lock + apply + notify + drop pattern

### 2. Duplicated run_cell logic across MCP and Tauri

`nb_run_cell` in `src-tauri/src/commands/notebook.rs` (lines 242–353) and `run_cell` in `server.rs` (lines 501–635) are nearly identical 100+ line functions. Both follow the same sequence: set status, prepare labels, lock session, evaluate, capture output, apply output, notify.

**Suggested refactoring:**
- Extract shared logic to `aximar-core` as a `CellEvaluator` or standalone `evaluate_cell()` async function
- Both MCP and Tauri commands call the shared implementation

### 3. Repeated lock acquisitions in notebook commands

`src-tauri/src/commands/notebook.rs` (353 lines) acquires `state.notebook.lock().await` up to 6 times in `nb_run_cell`:
1. Lock to read cell input
2. Lock to set Running status
3. Lock to get execution count and label context
4. Session lock for evaluation (separate mutex)
5. Lock to record label
6. Lock to set output or error

Each lock/unlock cycle adds overhead and creates windows for interleaving.

**Suggested refactoring:**
- Hold the notebook lock across consecutive notebook operations, only releasing before the session lock for Maxima I/O
- Or create a `NotebookSession` wrapper that batches operations under one guard

## High

### 4. config.rs mixes too many concerns (529 lines)

`src-tauri/src/commands/config.rs` handles theme/UI config (13 fields), Maxima backend config (5 fields), MCP server lifecycle control, config I/O, and WSL integration.

**Specific issues:**
- 44-line field-by-field config update cascade (`if let Some(x) = updates.x { config.x = x; }` repeated 17 times)
- 9 inline Arc clones when rebuilding the MCP server
- WSL command execution (`check_wsl_maxima`, `list_wsl_distros`) mixed with config code

**Suggested refactoring:**
- Use a macro or generic helper for field updates
- Split into `config_ui.rs`, `config_backend.rs`, `wsl.rs`
- Extract `fn clone_mcp_deps(state: &AppState)` for the repeated Arc cloning

### 5. Deep cloning in undo/redo snapshots

`crates/aximar-core/src/notebook.rs` (721 lines) deep-clones the entire `Vec<Cell>` on every undoable command. Each Cell includes `Vec<OutputEvent>` with timestamps and line content. With 50 max undo states, this can hold megabytes of cloned data.

**Specific locations:**
- `push_undo_snapshot()` clones `self.cells` on every structural command
- Undo/Redo operations clone cells again when swapping stacks

**Suggested refactoring:**
- Use `Arc<Vec<Cell>>` for copy-on-write sharing between snapshots
- Or store only mutation diffs instead of full snapshots
- Or exclude output data from snapshots (it can be re-derived)

### 6. MCP startup callback complexity

`src-tauri/src/mcp/startup.rs` (108 lines) builds a 3-layer nested closure (Fn + Arc + async move) for the notebook-change callback. The closure captures multiple Arc clones, spawns a tokio task, and uses `try_lock` which silently drops events on contention.

**Suggested refactoring:**
- Use a bounded channel: emit effects to a channel, consume them in a dedicated async task
- Or create a `NotebookChangeHandler` struct instead of a closure

## Medium

### 7. Parser is a single large state machine

`crates/aximar-core/src/maxima/parser.rs` (555 lines) — `parse_output()` is a 150+ line function with 7 mutable accumulators (`latex`, `error_lines`, `text_lines`, `skip_next_false`, `in_error`, `output_label`, `latex_buf`).

**Suggested refactoring:**
- Create a `ParseState` struct holding all accumulators
- Break the loop body into `process_line(&mut state, line)` and `finalise_output(&state)`

### 8. Protocol functions repeat the same pattern

`crates/aximar-core/src/maxima/protocol.rs` (156 lines) — `evaluate()`, `query_variables()`, `kill_variable()`, `kill_all_variables()` all build a Maxima command string, write to stdin, set up a timeout, read until a sentinel, and parse output.

**Suggested refactoring:**
- Abstract the pattern into a single `execute_command()` function
- Each command becomes a struct or enum variant providing its input string and sentinel

### 9. OutputEvent cloning in MultiOutputSink

`crates/aximar-core/src/maxima/output.rs` — `MultiOutputSink::emit()` clones the `OutputEvent` for each sink in the broadcast list. With high-throughput Maxima output this is wasteful.

**Suggested refactoring:**
- Use `Arc<OutputEvent>` instead of cloning per sink

## Summary

| File | Lines | Priority | Primary issues |
|------|-------|----------|----------------|
| `crates/aximar-mcp/src/server.rs` | 1057 | Critical | God file, repeated patterns, excessive locking |
| `src-tauri/src/commands/notebook.rs` | 353 | Critical | Duplicates MCP run_cell, repeated lock acquisitions |
| `src-tauri/src/commands/config.rs` | 529 | High | Boilerplate field updates, mixed concerns, Arc cloning |
| `crates/aximar-core/src/notebook.rs` | 721 | High | Deep cloning of cell snapshots for undo |
| `src-tauri/src/mcp/startup.rs` | 108 | High | Nested closure complexity, silent event loss |
| `crates/aximar-core/src/maxima/parser.rs` | 555 | Medium | Large monolithic function |
| `crates/aximar-core/src/maxima/protocol.rs` | 156 | Medium | Repeated command pattern |

## Refactoring approach

**Phase 1:** Extract shared `evaluate_cell()` to aximar-core, eliminating the duplication between MCP and Tauri. This also naturally reduces `server.rs` and `notebook.rs` command sizes.

**Phase 2:** Split `server.rs` into handler modules by domain. Centralise the lock-apply-notify pattern into a helper.

**Phase 3:** Batch notebook lock acquisitions in command handlers. Hold the lock across consecutive operations.

**Phase 4:** Reduce config boilerplate (macro or generic update) and split config.rs by concern.

**Phase 5:** Optimise undo snapshots (Arc sharing or diff-based) and OutputEvent broadcasting.
