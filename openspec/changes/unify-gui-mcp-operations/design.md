## Context

See proposal.md for why. Today both front ends already share `evaluation::evaluate_cell`, `session_ops::ensure_session` and `Notebook::apply`, which returns a `CommandEffect` for each change. Around those, each side has its own code:

- **Status effects.** `evaluate_cell` discards the Running effect and returns no effects on error. `nb_run_cell` re-applies Running and Error itself; MCP cannot.
- **Delivery to the window.** Tauri commands call `emit_notebook_state` directly with the `AppHandle` they receive, so their events are ordered. MCP in connected mode goes through callbacks in `src-tauri/src/mcp/startup.rs` that spawn a task per effect and emit only if `try_lock` on `Arc<Mutex<Option<AppHandle>>>` succeeds. Session status and raw output emitters use the same `try_lock`.
- **Trust.** Four call sites run `safety::detect_dangerous_calls` with different rules: `nb_run_cell` (trust flag), MCP `run_cell` (trust flag when connected, `allow_dangerous` otherwise), MCP `evaluate_expression` (always, unless `allow_dangerous`), and `batch.rs`.
- **Run-all.** The window loops in `Toolbar.tsx` over the store's cells; MCP loops over `run_cell` and re-reads its JSON; batch has a third loop.
- **Input preparation.** `evaluate_cell` applies Unicode translation and label rewriting; Tauri `evaluate_expression` applies labels only; MCP `evaluate_expression` applies Unicode only.
- **Paths.** Only `do_create_session` and `batch.rs` call `NotebookRegistry::set_path`. The window keeps `filePath` in its store and clears it on any `notebook_replaced`.
- **Settings.** The embedded `ServerCore` is built once with the default backend, no Maxima path and a 30 s timeout.
- **Catalog.** Tauri, MCP and the LSP each merge catalog and package results their own way.

## Goals / Non-Goals

**Goals:**
- Every operation both front ends offer has one entry point in core that does validation, policy and state changes, and reports effects.
- Tauri commands and MCP tools only translate parameters and results.
- Nothing that changes a notebook or session is invisible to the window.

**Non-Goals:**
- Merging `AximarMcpServer` and `SimpleMcpServer` beyond what follows from the shared core calls.
- Changing the `CommandEffect` payload the frontend receives (a full notebook snapshot per effect, as `docs/core-tauri-data-transfer.md` describes).

## Decisions

### 1. Effects are reported through a sink, as they happen

Core operations take an `&dyn EffectSink` (one method, `effect(notebook_id, CommandEffect)`) and call it after each `Notebook::apply`, including Running at the start of an evaluation and Error on every failure path. `evaluate_cell` keeps returning its result for the caller's reply, but no longer returns effects in a `Vec`.

- Tauri implements the sink with `emit_notebook_state`; connected MCP implements it with the channel below; standalone MCP and batch use a no-op sink.
- *Alternative:* return all effects in the result and let callers emit them. That is today's design, and it is why Running arrives late or never: a long evaluation can't return its first effect before it finishes.

### 2. One ordered channel to the window

`AppState` holds the `AppHandle` in a `OnceLock<AppHandle>` set in `setup`, replacing `Arc<Mutex<Option<AppHandle>>>`. Emitters call `get()` and never skip. Connected MCP sends `(notebook_id, effect)` pairs into an unbounded `tokio::mpsc` channel; one task drains it, locks the notebook, takes the snapshot and emits. Lifecycle events (created, closed, active changed) go through the same channel, so the window sees them in the order they happened. Session status needs no notebook lock, so its callback emits directly and synchronously.

- Tauri commands keep emitting directly: they already run in order on the command's task.
- *Alternative:* keep `tokio::Mutex` and use `lock().await` instead of `try_lock`. That fixes the drops but not the ordering between spawned tasks, and the snapshot would still be taken whenever the task happens to run.

### 3. Session settings come from a provider

`ServerCore` takes a `SessionSettingsProvider` (a closure returning a `SessionSettings` of backend, Maxima path and timeout) instead of fixed values, and calls it when a session starts and when a cell runs. The desktop app passes a closure that reads its config file, so changes in Settings apply to the next session MCP starts. Standalone MCP and batch pass fixed values read from the environment at startup.

`session_ops::ensure_session` and `spawn_and_init_session` take `&SessionSettings` too, and the window's commands read one `SessionSettings` from the config instead of three separate values.

### 4. Trust is one function with a mode

`safety::check_trust(notebook, input, packages, mode) -> TrustDecision` replaces the four checks. `mode` is `Interactive` (desktop app and connected MCP: blocked only when the notebook is untrusted) or `Headless { allow_dangerous }` (standalone MCP and batch: blocked unless `allow_dangerous`). `TrustDecision` is `Allowed` or `NeedsTrust { functions }`.

- Run cell, run-all and evaluate expression call it before evaluating.
- When an interactive run is blocked, core reports a `TrustRequired { cell_id, functions }` effect, so the window shows its banner whichever front end ran the cell. This is a new `CommandEffect` variant, reported by `evaluation::check_cell_trust`, which run cell, MCP `run_cell` and batch call. The window's event carries the functions in a `trust_functions` field; the frontend sets `pendingTrustFunctions` on that notebook from it, replacing the `useMaxima.ts` path, and leaves the cells alone.
- The trust flag itself stays where it is: `true` for new notebooks, `false` after a load, set by the window's banner.

### 5. Run-all lives in core

`evaluation::run_all(ctx, sink, settings, mode, cancel) -> RunAllReport` runs code cells in order and stops at the first error, trust block or failure. It reports each cell's outcome with the statuses MCP already uses (`ok`, `empty`, `error`, `needs_trust`, `failed`, `not_run`).

- MCP `run_all_cells` serialises the report as today. Batch maps it to its exit code.
- The window gets a `nb_run_all` command. Before calling it, the frontend flushes the notebook's pending edits; `dirty-inputs.ts` gains `flushNotebook(notebookId)`.
- Cancel stops the run after the current cell: `run_all` checks a flag between cells, and `cancel_evaluation` sets it.
- *Alternative:* keep the frontend loop and call core per cell. The loop would still decide when to stop on its own, which is the drift this change removes.

### 6. Input preparation in one function

`evaluation::prepare_input(input, label_ctx) -> Result<String, UnresolvedLabel>` applies Unicode translation, then label rewriting. Cells and both `evaluate_expression` commands use it. Expressions have no cell, so they get no bare-`%` context; `%oN` references still resolve.

### 7. Paths are recorded by core, and the window follows them

`NotebookRegistry::set_path` gains a `PathChanged { path }` effect, with `path` optional. MCP `open_notebook` and `save_notebook` set it; `load_template` clears it, since the notebook no longer matches the file. The window's open, save and save-as call a new `nb_set_path` command after their file operation. The frontend takes its tab title and `filePath` from `PathChanged` events, and `applyBackendState` stops clearing the path on `notebook_replaced`.

- Maxima's working directory comes from the path when a session starts (`session_ops.rs`). Setting a path on a running session takes effect at its next restart; the window can say so.
- File conversion stays where it is until S1 (#16). This decision only moves the path.

### 8. Catalog queries in one core API

`catalog::queries` offers `search`, `docs` and `complete`, merging the built-in catalog and installed packages, deduplicating names and suggesting near matches for misses. Tauri and MCP serialise the results. The LSP keeps its own code for now (proposal non-goals).

## Risks / Trade-offs

- [Run All moves from the frontend loop to a backend command, so the window can't stop between cells by itself] → The running cell's stop button already calls `cancel_evaluation`, which now also sets the run-all flag (decision 5), so stopping a cell stops the run.
- [An unbounded channel can grow if the window stops draining it] → Effects are small and bounded by user actions; the drain task does nothing but lock, snapshot and emit.
- [A new `CommandEffect` variant and a new event shape the frontend must handle] → Add both in the same PR as their emitters, with vitest coverage for the store's handling.
- [`evaluate_expression` in connected mode now runs dangerous calls in trusted notebooks] → That is the rule `run_cell` already follows; the changelog notes it.

## Migration Plan

Each task group in `tasks.md` is one pull request and leaves both front ends working. Groups 1 and 2 come first because later groups report through the sink and channel. No data migration: saved notebooks and MCP tool shapes are unchanged except the trust rule. Rolling back a group means reverting its PR.
