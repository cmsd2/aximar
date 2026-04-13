# Backend Code Quality

Audit performed 2026-03-24 against `master` at commit `8d6f1c3`.
Updated 2026-03-25 after Phase 1 + Phase 2 refactoring.
Updated 2026-04-13 after tool crate review (maxima-dap, maxima-lsp, aximar-core).

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
- **Result: 1,057 → ~1,300 lines** (tool count grew; supporting code moved out)

**What remains:**
- The `#[tool_router]` macro requires all tool methods in a single impl block, so splitting tools across files is not feasible without rmcp changes
- Three documentation tools still have near-identical JSON serialisation patterns
- `save_notebook`, `open_notebook`, `load_template` share lock-apply-notify boilerplate that could be extracted into a helper
- The same catalog/documentation tool handlers are duplicated between `AximarMcpServer` and `SimpleMcpServer` — both delegate to `self.core.do_*` but repeat the plumbing

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

### 4. DapServer is a god object (18 fields, 6 files) — Outstanding

`crates/maxima-dap/src/server/mod.rs` combines too many responsibilities:
DAP protocol dispatch, Maxima process management, breakpoint lifecycle,
variable inspection, output filtering, stack frame caching, and debug
state transitions — all on a single struct with 18+ methods spread across
6 sub-modules.

**Specific issues:**
- `StrategyContext` is constructed identically 8+ times across breakpoints.rs, communication.rs, and inspection.rs — always `StrategyContext { process, state: &self.state, source_index: &self.source_index }`
- `handle_continue`, `handle_next`, `handle_step_in` have near-identical 3-way match arms (Debugger → update state + refresh + flush + send stopped; Normal → terminate; Err → report + terminate)
- `suppress_output` flag is set/cleared in 4 different files (lifecycle.rs, breakpoints.rs, inspection.rs ×2) with no guard against double-suppression or missed cleanup
- `cached_frames` and `cached_frame_args` are parallel vectors with no type-level guarantee they stay in sync

**Suggested refactoring:**
- Extract `fn strategy_context(&mut self) -> Result<StrategyContext, AppError>` helper
- Extract `fn handle_execution_result(&mut self, result, reason)` for the 3-way match
- Consider a `SuppressGuard` RAII pattern for output suppression
- Group parallel vectors into a `CachedBacktrace { frames, args }` struct

## High

### 5. config.rs mixes too many concerns (624 lines) — Outstanding

`src-tauri/src/commands/config.rs` handles theme/UI config (13 fields), Maxima backend config (5 fields), MCP server lifecycle control, config I/O, and WSL integration.

**Specific issues:**
- 44-line field-by-field config update cascade (`if let Some(x) = updates.x { config.x = x; }` repeated 17 times)
- 9 inline Arc clones when rebuilding the MCP server
- WSL command execution (`check_wsl_maxima`, `list_wsl_distros`) mixed with config code

**Suggested refactoring:**
- Use a macro or generic helper for field updates
- Split into `config_ui.rs`, `config_backend.rs`, `wsl.rs`
- Extract `fn clone_mcp_deps(state: &AppState)` for the repeated Arc cloning

### 6. Deep cloning in undo/redo snapshots (758 lines) — Outstanding

`crates/aximar-core/src/notebook.rs` deep-clones the entire `Vec<Cell>` on every undoable command. Each Cell includes `Vec<OutputEvent>` with timestamps and line content. With 50 max undo states, this can hold megabytes of cloned data.

**Specific locations:**
- `push_undo_snapshot()` clones `self.cells` on every structural command
- Undo/Redo operations clone cells again when swapping stacks

**Suggested refactoring:**
- Exclude output data from snapshots (store only id, cell_type, input) — ~50% memory savings
- Use `VecDeque` instead of `Vec` for O(1) removal when exceeding MAX_UNDO
- Optionally use `Arc<Vec<Cell>>` for copy-on-write sharing between snapshots

### 7. MCP startup callback complexity (158 lines) — Outstanding

`src-tauri/src/mcp/startup.rs` builds a 3-layer nested closure (Fn + Arc + async move) for the notebook-change callback. The closure captures multiple Arc clones, spawns a tokio task, and uses `try_lock` which silently drops events on contention.

**Suggested refactoring:**
- Use a bounded channel: emit effects to a channel, consume them in a dedicated async task
- Or create a `NotebookChangeHandler` struct instead of a closure

### 8. Duplicated path validation in parser.rs — Outstanding

`crates/aximar-core/src/maxima/parser.rs` has `is_safe_svg_path` and `is_safe_plotly_path` that are 95% identical — both canonicalize the path, check against `std::env::temp_dir()` and `backend.host_temp_dir()`, differing only in the allowed file extension.

**Suggested refactoring:**
```rust
fn is_safe_file_path(path_str: &str, backend: &Backend, allowed_ext: &str) -> bool
```

### 9. LSP lookup chain duplicated across hover and signature — Outstanding

`crates/maxima-lsp/src/hover.rs` and `signature.rs` both implement the
same 4-level lookup: full docs → catalog → document symbols → packages.
Each is ~80 lines with the same structure.

**Suggested refactoring:**
- Extract a shared `lookup_function(name, docs, catalog, documents, packages)` function
  returning a unified `FunctionInfo` struct

## Medium

### 10. Parser is a single large state machine (~1,100 lines) — Outstanding

`crates/aximar-core/src/maxima/parser.rs` — `parse_output()` is a 150+ line function with 8 mutable accumulators (`latex`, `error_lines`, `text_lines`, `skip_next_false`, `in_error`, `output_label`, `latex_buf`, `in_verbatim`).

**Suggested refactoring:**
- Create a `ParseState` struct holding all accumulators
- Break the loop body into `process_line(&mut state, line)` and `finalise_output(&state)`

### 11. Protocol functions repeat the same pattern (196 lines) — Outstanding

`crates/aximar-core/src/maxima/protocol.rs` — `evaluate()`, `query_variables()`, `kill_variable()`, `kill_all_variables()` all build a Maxima command string, write to stdin, set up a timeout, read until a sentinel, and parse output.

**Suggested refactoring:**
- Abstract the pattern into a single `execute_command()` function
- Each command becomes a struct or enum variant providing its input string and sentinel

### 12. OutputEvent cloning in MultiOutputSink — Outstanding

`crates/aximar-core/src/maxima/output.rs` — `MultiOutputSink::emit()` clones the `OutputEvent` for each sink in the broadcast list. With high-throughput Maxima output this is wasteful.

**Suggested refactoring:**
- Use `Arc<OutputEvent>` instead of cloning per sink

### 13. Regex recompilation in strategy_enhanced.rs — Outstanding

`crates/maxima-dap/src/strategy_enhanced.rs` compiles `Regex::new(r"Bkpt\s+(\d+)")` and several other patterns inside both `parse_enhanced_breakpoint_response` and `parse_breakpoint_resolutions`. These are called on every breakpoint operation.

**Suggested refactoring:**
- Use `LazyLock<Regex>` statics (as already done in server/protocol.rs)

### 14. LSP repeated word-at-position extraction — Outstanding

`crates/maxima-lsp/src/server.rs` repeats the same `get document → extract word → bail if None` pattern 5 times across hover, goto_definition, references, signature_help, and completion handlers.

**Suggested refactoring:**
```rust
fn get_word_at_position(&self, uri: &Url, pos: Position) -> Option<String> {
    self.documents.get(uri)
        .and_then(|doc| helpers::word_at_position(&doc.content, pos.line, pos.character))
}
```

### 15. Inconsistent error types in aximar-core — Outstanding

The crate mixes `Result<T, AppError>`, `Result<T, String>` (notebooks/io.rs), and `.expect()` panics (notebooks/data.rs template loading). Lock poisoning is silently ignored in log.rs and capture.rs.

**Suggested refactoring:**
- Standardise on `AppError` throughout — add variants for serialization and I/O errors
- Replace `.expect()` on embedded JSON with `Result` returns
- At minimum, log when lock acquisition fails

### 16. No timeout on debugger command reads — Outstanding

`crates/maxima-dap/src/server/communication.rs` — `send_debugger_command` and `send_debugger_command_raw` have no timeout, unlike `send_maxima_and_wait` which has configurable timeout logic. A malformed Maxima response could hang the debug session indefinitely.

**Suggested refactoring:**
- Apply the same timeout pattern from `send_maxima_and_wait`
- Or add a global read timeout on the process I/O layer

## Summary

| File | Audit | Now | Status |
|------|-------|-----|--------|
| `crates/aximar-mcp/src/server.rs` | 1,057 | ~1,300 | Partially resolved (params, convert, run_cell extracted) |
| `src-tauri/src/commands/notebook.rs` | 353 | 351 | Resolved (nb_run_cell is thin wrapper) |
| `crates/aximar-core/src/evaluation.rs` | — | 149 | New — shared evaluate_cell() |
| `crates/aximar-mcp/src/params.rs` | — | 164 | New — parameter types and helpers |
| `crates/aximar-mcp/src/convert.rs` | — | 86 | New — notebook format conversion |
| `crates/maxima-dap/src/server/mod.rs` | — | 225 | Outstanding (god object) |
| `crates/maxima-dap/src/server/breakpoints.rs` | — | 491 | Outstanding (long functions) |
| `crates/maxima-dap/src/server/execution.rs` | — | 250 | Outstanding (duplicated match arms) |
| `crates/maxima-dap/src/strategy_enhanced.rs` | — | 435 | Outstanding (regex recompilation) |
| `crates/maxima-lsp/src/server.rs` | — | 335 | Outstanding (repeated patterns) |
| `src-tauri/src/commands/config.rs` | 529 | 624 | Outstanding |
| `crates/aximar-core/src/notebook.rs` | 721 | 758 | Outstanding |
| `src-tauri/src/mcp/startup.rs` | 108 | 158 | Outstanding |
| `crates/aximar-core/src/maxima/parser.rs` | 555 | ~1,100 | Outstanding |
| `crates/aximar-core/src/maxima/protocol.rs` | 156 | 196 | Outstanding |

## Test coverage

The MCP server now has 26 integration tests (`crates/aximar-mcp/src/server_tests.rs`):
- **20 tests** run by default — catalog search, docs, packages, notebook lifecycle, cell CRUD, move/delete, templates, save/open, session status, logs
- **6 tests** require Maxima (`#[ignore]`) — run_cell, run_all_cells, evaluate_expression, list/kill variables, restart session
- Run with `cargo test -p aximar-mcp` (default) or `cargo test -p aximar-mcp -- --ignored` (all)

maxima-dap has 25 unit tests + 47 protocol tests + 20 integration tests:
- Unit tests cover breakpoint parsing, frame parsing, strategy response parsing
- Protocol tests cover DAP message serialization/deserialization
- Integration tests require Maxima (`#[ignore]`) — breakpoints, stepping, backtrace, evaluate, error recovery
- Run with `cargo test -p maxima-dap` (unit + protocol) or `cargo test -p maxima-dap -- --ignored` (all)

**Under-tested modules** (no unit tests):
- `aximar-core/src/maxima/process.rs` (779 lines)
- `aximar-core/src/maxima/parser.rs` (~1,100 lines)
- `aximar-core/src/maxima/protocol.rs` (196 lines)
- `aximar-core/src/maxima/backend.rs` (217 lines)
- `aximar-core/src/catalog/search.rs` (581 lines)

## Refactoring approach

**Phase 1 — Done:** Extracted shared `evaluate_cell()` to aximar-core, eliminating MCP/Tauri duplication and consolidating lock acquisitions from 6–7 to 2+1.

**Phase 2 — Done:** Extracted parameter types to `params.rs`, notebook conversion to `convert.rs`, deduplicated constructors via `build()`. Added integration test suite.

**Phase 3:** Reduce config boilerplate (macro or generic update) and split config.rs by concern.

**Phase 4:** Optimise undo snapshots (lightweight snapshots + VecDeque) and OutputEvent broadcasting.

**Phase 5:** Refactor parser into `ParseState` struct and protocol into `execute_command()` abstraction.

**Phase 6:** DapServer refactoring — extract execution response handler, StrategyContext helper, SuppressGuard, and CachedBacktrace struct.

**Phase 7:** LSP deduplication — extract word-at-position helper and shared lookup function.
