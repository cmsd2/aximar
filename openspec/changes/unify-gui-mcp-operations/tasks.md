Each numbered group is one pull request. Every group updates the Unreleased section of `CHANGELOG.md` for changes MCP clients can see, and runs `cargo test --workspace`, `cargo test -p aximar-mcp -- --ignored` (with Maxima) and `npm test`.

## 1. Ordered delivery to the window (#55)

- [x] 1.1 Replace `Arc<Mutex<Option<AppHandle>>>` in `AppState` with `OnceLock<AppHandle>` set in `setup`, and remove every `try_lock` on it; verify with `git grep try_lock src-tauri` returning nothing
- [x] 1.2 Send connected-MCP notebook effects and lifecycle events through one `tokio::mpsc` channel drained by a single task that snapshots and emits; session status emits directly, now that it takes no lock; verify with a unit test in `src-tauri` that drains 50 queued effects into a recording emitter in order
- [x] 1.3 Emit an `active_changed` lifecycle event from MCP `switch_notebook` (MCP create and close already notify; the window's own create, close and switch update that window directly); verify with an aximar-mcp test whose connected-mode callbacks record a `created` then an `active_changed` event
- [x] 1.4 Handle `active_changed` in `useNotebookEvents.ts`; verify with a vitest test that the store's active tab follows the event

## 2. Effects reported as they happen (#54)

- [ ] 2.1 Add an `EffectSink` trait in aximar-core and pass it to `evaluate_cell`; report Running at start and Error on every failure path; verify with a core unit test that a markdown cell and an empty cell report no Running, and a test with a recording sink
- [ ] 2.2 Implement the sink for Tauri (`emit_notebook_state`), connected MCP (the group 1 channel), and standalone MCP and batch (no-op); remove the Running and Error workaround in `nb_run_cell`; verify with a Maxima-backed aximar-mcp test in connected mode that records Running before Success for `2 + 3`, and Running before Error for `1/0`

## 3. Session settings from the app's config (#53)

- [ ] 3.1 Give `ServerCore` a `SessionSettingsProvider` in place of fixed backend, Maxima path and timeout; standalone MCP passes its command-line values; verify with an aximar-mcp test (no Maxima) where the provider names a missing Maxima path and `run_cell` fails with that path in the error
- [ ] 3.2 Pass a provider from `src-tauri/src/mcp/startup.rs` that reads `read_backend`, `read_maxima_path` and `read_eval_timeout` when a session starts; verify with a unit test of the provider against a config file

## 4. One trust check, with the banner for MCP runs (#39)

- [ ] 4.1 Add `safety::check_trust(notebook, input, packages, mode)` with `Interactive` and `Headless { allow_dangerous }` modes; verify with core unit tests covering trusted and untrusted notebooks in each mode
- [ ] 4.2 Use it in `nb_run_cell`, MCP `run_cell`, both `evaluate_expression` commands and `batch.rs`, deleting their own checks; verify with aximar-mcp tests: a trusted connected notebook runs an expression that calls `system`, an untrusted one blocks it, and standalone without `--allow-dangerous` blocks it
- [ ] 4.3 Report a `TrustRequired { cell_id, functions }` effect when an interactive run is blocked; the frontend sets `pendingTrustFunctions` from it and `useMaxima.ts` stops setting it; verify with an aximar-mcp connected-mode test that records the effect, and a vitest test that the store shows the banner's functions

## 5. Run-all in core (#41)

- [ ] 5.1 Add `evaluation::run_all` returning a per-cell report (`ok`, `empty`, `error`, `needs_trust`, `failed`, `not_run`) that stops at the first error, trust block or failure, and checks a cancel flag between cells; verify with Maxima-backed core tests for an error in the middle, a trust block in the middle and a cancel
- [ ] 5.2 Switch MCP `run_all_cells` and `batch.rs` to `run_all`, keeping the tool's result shape; verify the existing `run_all_cells` tests pass unchanged
- [ ] 5.3 Add a `nb_run_all` Tauri command; the toolbar calls it after `flushNotebook(notebookId)` (new in `dirty-inputs.ts`) and drops its own loop; `cancel_evaluation` sets the run-all cancel flag; verify with a vitest test that `flushNotebook` sends only that notebook's edits, and by hand that Run All stops at `1/0` and at a stopped cell

## 6. One input preparation

- [ ] 6.1 Add `evaluation::prepare_input` (Unicode translation, then label rewriting) and use it for cells and both `evaluate_expression` commands; verify with core unit tests that `α + 1` becomes `alpha + 1` and an unknown `%o9` is reported, for cells and expressions

## 7. Notebook paths (#40)

- [ ] 7.1 Add a `PathChanged { path }` effect for `NotebookRegistry::set_path`, with `path` optional; MCP `open_notebook` and `save_notebook` set it and `load_template` clears it; verify with aximar-mcp tests (no Maxima) that `list_notebooks` reports the path and title after open and save, and none after a template load
- [ ] 7.2 Add a `nb_set_path` Tauri command; the window's open, save and save-as call it, the store takes `filePath` and the title from `PathChanged`, and `applyBackendState` no longer clears the path on `notebook_replaced`; verify with vitest tests that a `PathChanged` event sets the title and that `notebook_replaced` keeps it
- [ ] 7.3 Check that a session started after opening `work/calc.macnb` runs in `work/`; verify with a Maxima-backed aximar-mcp test that loads `helpers.mac` from beside the notebook

## 8. Catalog queries in core

- [ ] 8.1 Add `catalog::queries` with `search`, `docs` and `complete` that merge the catalog and installed packages, deduplicate names and suggest near matches; verify with core unit tests for a name in both sources and for a miss
- [ ] 8.2 Use it in the Tauri catalog commands and the MCP catalog tools, which only format results; verify the existing MCP catalog tests pass and Tauri `complete_function` no longer returns a package name twice

## 9. Documentation and close-out

- [ ] 9.1 Update `docs/mcp-server.md` (trust rule, path behaviour), `docs/core-tauri-data-transfer.md` (the channel and the sink) and the status of the affected bugs in `docs/code-audit.md`; verify the docs name no removed function
- [ ] 9.2 Archive this change with `openspec archive unify-gui-mcp-operations` once groups 1 to 8 have merged, and check that `openspec/specs/` gains `cell-execution`, `notebook-files` and `gui-updates`
