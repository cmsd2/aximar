## Why

The desktop app and the MCP server implement several notebook operations separately, and the copies have drifted. An MCP client and a user at the window see different behaviour for the same notebook: runs over MCP show no Running or error status in the window, cells blocked by trust give no banner, notebooks opened over MCP stay "Untitled", and the embedded MCP server starts Maxima with the wrong settings. Issue #41 asks for every operation both sides offer to go through one function in aximar-core. The audit in `docs/code-audit.md` found more copies than #41 listed.

## What Changes

- **Changes reach the window in order and without loss.** Effects from MCP go through one ordered channel instead of a spawned task per effect, and emitters stop skipping events when a lock is busy (#55). Creating, closing and switching notebooks also notify the window.
- **Cell runs report every status change as it happens.** Evaluating a cell reports Running when it starts and the result or error when it ends, whichever front end started it. The Tauri command's own status workaround goes away (#54).
- **Sessions use the configured Maxima.** The embedded MCP server starts Maxima with the backend, Maxima path and timeout from the app's settings, read when the session starts (#53).
- **One trust rule.** Running a cell, running all cells and evaluating an expression apply the same trust check. In the desktop app and connected MCP, the notebook's trust flag decides; standalone MCP keeps `--allow-dangerous`. A cell blocked over MCP shows the trust banner in the window (#39).
- **One run-all.** Run All in the window and `run_all_cells` over MCP share one implementation in core that stops at the first error, trust block or failure and reports each cell's outcome (#41).
- **One input preparation.** Unicode translation and label rewriting happen in one place for cells and expressions, from either front end.
- **Notebooks keep their file path.** Opening or saving from either front end records the path in the notebook registry, updates the tab title, and sets Maxima's working directory for that notebook (#40).
- **One catalog query layer.** Function search, documentation and completion come from one core API that merges the built-in catalog and installed packages; the front ends only format results.
- **BREAKING (MCP, connected mode):** `evaluate_expression` follows the notebook's trust flag like `run_cell`, instead of blocking dangerous calls in trusted notebooks.

## Capabilities

### New Capabilities

- `cell-execution`: running cells, running all cells and evaluating expressions, from either front end: trust, statuses, stopping rules and the Maxima settings a session uses.
- `notebook-files`: what a notebook records when it is opened or saved, and what the window and MCP report about it.
- `gui-updates`: how changes to notebooks and sessions, from any source, reach the desktop window.

### Modified Capabilities

None. `label-references` keeps its requirements; this change only moves where its rewriting is called from.

## Impact

- **Layers:** core (`evaluation.rs`, `session_ops.rs`, `registry.rs`, `safety.rs`, `catalog/`), MCP (`server.rs`, `batch.rs`, `simple_server.rs`), Tauri (`commands/`, `mcp/startup.rs`, `tauri_output.rs`, `lib.rs`), frontend (`Toolbar.tsx`, `useMaxima.ts`, `App.tsx`, `useNotebookEvents.ts`, `notebookStore.ts`).
- **Issues:** closes #39, #40, #41, #53, #54 and #55.
- **MCP clients:** tool names and result shapes stay the same, apart from the `evaluate_expression` trust rule above.
- **Saved notebooks:** no change to the file format.

## Non-goals

- Moving notebook file conversion into core, and fixing what the format loses (#7, #9, #10, #6). That is S1 (#16).
- Enhanced error explanations in the window (#56), which need a new field on stored outputs and belong with S1.
- Typed errors in place of `AppError::CommunicationError` for notebook commands. Worth doing, but separate.
- The language server's catalog queries. It can adopt the core API later.
- Undo and redo over MCP, and a trust tool for MCP clients.
