# Backend Code Quality

Audit performed 2026-03-24 against `master` at commit `8d6f1c3`.
Updated 2026-03-25 after Phase 1 + Phase 2 refactoring.

## Critical

### 1. server.rs is a god file ~~(1,057 lines)~~ — Partially resolved

`crates/aximar-mcp/src/server.rs` implements all 21 MCP tools as methods on a single struct.

**What was done:**
- Extracted parameter types, helpers, and `CellSummary` to `params.rs` (164 lines)
- Extracted notebook format conversion (`notebook_to_ipynb`, `ipynb_to_cell_tuples`) to `convert.rs` (86 lines)
- Deduplicated constructors: `new` and `new_connected` now delegate to shared `build()` method
- Extracted `run_cell` logic to shared `evaluate_cell()` in aximar-core (see #2)
- Removed unnecessary `run_cell_impl` indirection
- Added 26 integration tests in `server_tests.rs` (20 run by default, 6 require Maxima)
- **Result: 1,057 → 1,036 lines** (supporting code moved out; tool impl block unchanged)

**What remains:**
- The `#[tool_router]` macro requires all tool methods in a single impl block, so splitting tools across files is not feasible without rmcp changes
- Three documentation tools still have near-identical JSON serialisation patterns
- `save_notebook`, `open_notebook`, `load_template` share lock-apply-notify boilerplate that could be extracted into a helper

### 2. Duplicated run_cell logic across MCP and Tauri — Resolved

~~`nb_run_cell` in `src-tauri/src/commands/notebook.rs` and `run_cell` in `server.rs` are nearly identical 100+ line functions.~~

**What was done:**
- Created `crates/aximar-core/src/evaluation.rs` (149 lines) with shared `evaluate_cell()` function
- Both `nb_run_cell` (Tauri) and `run_cell` (MCP) are now thin wrappers (~15–20 lines each)
- Added `CellIsMarkdown`, `EmptyInput`, `CellNotFound` error variants to `AppError`
- Effects returned via `Vec<CommandEffect>` for transport-agnostic notification

### 3. Repeated lock acquisitions in notebook commands — Resolved

~~`nb_run_cell` acquires `notebook.lock().await` up to 6 times per call.~~

**What was done:**
- `evaluate_cell()` consolidates locks from 6–7 down to 2 notebook locks + 1 session lock:
  1. **Lock 1:** read cell input + validate + set Running + get exec count + build label context
  2. **Session lock:** evaluate via Maxima
  3. **Lock 2:** record label + apply output (or set error status)

## High

### 4. config.rs mixes too many concerns (624 lines) — Outstanding

`src-tauri/src/commands/config.rs` handles theme/UI config (13 fields), Maxima backend config (5 fields), MCP server lifecycle control, config I/O, and WSL integration.

**Specific issues:**
- 44-line field-by-field config update cascade (`if let Some(x) = updates.x { config.x = x; }` repeated 17 times)
- 9 inline Arc clones when rebuilding the MCP server
- WSL command execution (`check_wsl_maxima`, `list_wsl_distros`) mixed with config code

**Suggested refactoring:**
- Use a macro or generic helper for field updates
- Split into `config_ui.rs`, `config_backend.rs`, `wsl.rs`
- Extract `fn clone_mcp_deps(state: &AppState)` for the repeated Arc cloning

### 5. Deep cloning in undo/redo snapshots (758 lines) — Outstanding

`crates/aximar-core/src/notebook.rs` deep-clones the entire `Vec<Cell>` on every undoable command. Each Cell includes `Vec<OutputEvent>` with timestamps and line content. With 50 max undo states, this can hold megabytes of cloned data.

**Specific locations:**
- `push_undo_snapshot()` clones `self.cells` on every structural command
- Undo/Redo operations clone cells again when swapping stacks

**Suggested refactoring:**
- Exclude output data from snapshots (store only id, cell_type, input) — ~50% memory savings
- Use `VecDeque` instead of `Vec` for O(1) removal when exceeding MAX_UNDO
- Optionally use `Arc<Vec<Cell>>` for copy-on-write sharing between snapshots

### 6. MCP startup callback complexity (158 lines) — Outstanding

`src-tauri/src/mcp/startup.rs` builds a 3-layer nested closure (Fn + Arc + async move) for the notebook-change callback. The closure captures multiple Arc clones, spawns a tokio task, and uses `try_lock` which silently drops events on contention.

**Suggested refactoring:**
- Use a bounded channel: emit effects to a channel, consume them in a dedicated async task
- Or create a `NotebookChangeHandler` struct instead of a closure

## Medium

### 7. Parser is a single large state machine (616 lines) — Outstanding

`crates/aximar-core/src/maxima/parser.rs` — `parse_output()` is a 150+ line function with 8 mutable accumulators (`latex`, `error_lines`, `text_lines`, `skip_next_false`, `in_error`, `output_label`, `latex_buf`, `in_verbatim`).

**Suggested refactoring:**
- Create a `ParseState` struct holding all accumulators
- Break the loop body into `process_line(&mut state, line)` and `finalise_output(&state)`

### 8. Protocol functions repeat the same pattern (196 lines) — Outstanding

`crates/aximar-core/src/maxima/protocol.rs` — `evaluate()`, `query_variables()`, `kill_variable()`, `kill_all_variables()` all build a Maxima command string, write to stdin, set up a timeout, read until a sentinel, and parse output.

**Suggested refactoring:**
- Abstract the pattern into a single `execute_command()` function
- Each command becomes a struct or enum variant providing its input string and sentinel

### 9. OutputEvent cloning in MultiOutputSink — Outstanding

`crates/aximar-core/src/maxima/output.rs` — `MultiOutputSink::emit()` clones the `OutputEvent` for each sink in the broadcast list. With high-throughput Maxima output this is wasteful.

**Suggested refactoring:**
- Use `Arc<OutputEvent>` instead of cloning per sink

## Summary

| File | Audit | Now | Status |
|------|-------|-----|--------|
| `crates/aximar-mcp/src/server.rs` | 1,057 | 1,036 | Partially resolved (params, convert, run_cell extracted) |
| `src-tauri/src/commands/notebook.rs` | 353 | 351 | Resolved (nb_run_cell is thin wrapper) |
| `crates/aximar-core/src/evaluation.rs` | — | 149 | New — shared evaluate_cell() |
| `crates/aximar-mcp/src/params.rs` | — | 164 | New — parameter types and helpers |
| `crates/aximar-mcp/src/convert.rs` | — | 86 | New — notebook format conversion |
| `src-tauri/src/commands/config.rs` | 529 | 624 | Outstanding |
| `crates/aximar-core/src/notebook.rs` | 721 | 758 | Outstanding |
| `src-tauri/src/mcp/startup.rs` | 108 | 158 | Outstanding |
| `crates/aximar-core/src/maxima/parser.rs` | 555 | 616 | Outstanding |
| `crates/aximar-core/src/maxima/protocol.rs` | 156 | 196 | Outstanding |

## Test coverage

The MCP server now has 26 integration tests (`crates/aximar-mcp/src/server_tests.rs`):
- **20 tests** run by default — catalog search, docs, packages, notebook lifecycle, cell CRUD, move/delete, templates, save/open, session status, logs
- **6 tests** require Maxima (`#[ignore]`) — run_cell, run_all_cells, evaluate_expression, list/kill variables, restart session
- Run with `cargo test -p aximar-mcp` (default) or `cargo test -p aximar-mcp -- --ignored` (all)

## Refactoring approach

**Phase 1 — Done:** Extracted shared `evaluate_cell()` to aximar-core, eliminating MCP/Tauri duplication and consolidating lock acquisitions from 6–7 to 2+1.

**Phase 2 — Done:** Extracted parameter types to `params.rs`, notebook conversion to `convert.rs`, deduplicated constructors via `build()`. Added integration test suite.

**Phase 3:** Reduce config boilerplate (macro or generic update) and split config.rs by concern.

**Phase 4:** Optimise undo snapshots (lightweight snapshots + VecDeque) and OutputEvent broadcasting.

**Phase 5:** Refactor parser into `ParseState` struct and protocol into `execute_command()` abstraction.
