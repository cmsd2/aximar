# Aximar code audit, November 2026

Scope: everything except `crates/maxima-dap`, at `origin/master` (51812af). The DAP cleanup on `perf/dap-command-latency` touches two core files (`debugger.rs`, `process.rs`); it adds a stderr-drain parameter and Windows paths in `CANONICAL_LOC_RE`, and fixes none of the findings below.

Method: four read-only passes (core; MCP and Tauri adapters; frontend; LSP, mac parser and catalog-gen), then spot checks of every high finding against the code. Nothing was built or run. "Inferred" marks a consequence read from code but not reproduced.

| Area | Files | Lines | Tests |
|---|---|---|---|
| aximar-core | 46 | 13,100 | ~320 |
| aximar-mcp | 13 | 4,160 | 43 unit (9 ignored) + 6 ignored integration |
| src-tauri | 22 | 2,720 | 2 |
| frontend `src/` | 68 | 9,700 | 0, no test runner |
| maxima-lsp | 12 | 1,500 | 14 |
| maxima-mac-parser | 4 | 1,800 | 50 |
| catalog-gen | 6 | 3,400 | 31 |

## 1. Bugs found

Each bug links to the issue that tracks it, and the fixed ones link to the PR that fixed them: [PR #62](https://github.com/cmsd2/aximar/pull/62) for the quick fixes, and the OpenSpec change `unify-gui-mcp-operations` (#64 to #74, archived in `openspec/changes/archive/`) for bugs 9, 10, 11 and 13. 16 to 19 have no issue yet.

### Data loss or wrong results

1. **A debounced edit can go to the wrong notebook.** `src/lib/dirty-inputs.ts` keys pending edits by cell id only, and `nbUpdateCellInput` reads the active notebook when the 300 ms timer fires (`notebook-commands.ts:83`). Switching tabs does not flush, and `flushAll` has no callers. The backend replies "Cell not found", which only reaches `console.warn`; inferred, the edit is lost and a following run uses the old input. Tracked in [#47](https://github.com/cmsd2/aximar/issues/47). Fixed in [#62](https://github.com/cmsd2/aximar/pull/62).
2. **A config parse error wipes the user's settings.** `src-tauri/src/commands/config.rs:318` falls back to `AppConfig::default()` on bad JSON, and `set_config` then writes the defaults back, including a new `mcp_token`. `gemini_mcp_configure` (`config.rs:741`) does the same to `~/.gemini/settings.json`. Tracked in [#46](https://github.com/cmsd2/aximar/issues/46). Fixed in [#62](https://github.com/cmsd2/aximar/pull/62).
3. **Docker notebooks share one container name.** `process.rs:297` names the container `aximar-{pid}`, but each notebook has its own session. A second notebook's `docker run --name` fails, and `kill()` (`process.rs:1374`) removes the shared name, which can stop another notebook's container. Tracked in [#48](https://github.com/cmsd2/aximar/issues/48). Fixed in [#62](https://github.com/cmsd2/aximar/pull/62).
4. **Output capture race.** `evaluation.rs:75` clears the capture sink before taking the session lock, so a run queued behind an active one clears the first cell's partial output. MCP `evaluate_expression` has the same order (`server.rs:315`). Tracked in [#51](https://github.com/cmsd2/aximar/issues/51). Fixed in [#62](https://github.com/cmsd2/aximar/pull/62).
5. **Labels are rewritten inside strings.** `replace_bare_percent` (`labels.rs:52`) turns `"hello %"` into `"hello %o6"`; the test at `labels.rs:292` asserts this. Tracked in [#50](https://github.com/cmsd2/aximar/issues/50). Fixed in [#62](https://github.com/cmsd2/aximar/pull/62).
6. **An unclosed bracket hides the rest of a `.mac` file.** `skip_to_terminator` (`maxima-mac-parser/src/parser.rs:121`) ignores `;` and `$` while a bracket is open, so the outline, LSP symbols and DAP lookups lose every later definition, with no diagnostic. Tracked in [#57](https://github.com/cmsd2/aximar/issues/57).
7. **FindBar Replace All corrupts overlapping matches.** `FindBar.tsx:49` advances by `idx + 1`, and Replace All splices the overlapping ranges: "aa" to "b" in "aaa" gives "b" (worked by hand). Tracked in [#52](https://github.com/cmsd2/aximar/issues/52). Fixed in [#62](https://github.com/cmsd2/aximar/pull/62).

### Things that silently don't work

8. **Enhanced error explanations never reach the screen.** Both output paths compute `error_info`, but core's `CellOutput` (`notebook.rs:26`) has no such field, and `useNotebookEvents.ts:73` sets `errorInfo: null`. `EnhancedErrorOutput` always shows its raw fallback. Lost in commit abb3fc9. Tracked in [#56](https://github.com/cmsd2/aximar/issues/56). Fixed in [#82](https://github.com/cmsd2/aximar/pull/82): `CellOutput` carries `error_info`, for live runs and for errors read from a file.
9. **The embedded MCP server ignores the user's Maxima settings.** `src-tauri/src/mcp/startup.rs:84` hard-codes the default backend, no Maxima path and a 30 s timeout. If an MCP call starts the session first, WSL, Docker and custom-path users get the wrong Maxima. Tracked in [#53](https://github.com/cmsd2/aximar/issues/53). Fixed in [#67](https://github.com/cmsd2/aximar/pull/67).
10. **The GUI misses status for MCP-driven runs.** `evaluate_cell` discards the Running and error effects (`evaluation.rs:55`, and the error returns). Tauri re-applies them itself (`commands/notebook.rs:346-381`); MCP `run_cell` cannot, so in connected mode the GUI never sees Running, errors or timeouts from MCP runs (inferred). Tracked in [#54](https://github.com/cmsd2/aximar/issues/54). Fixed in [#66](https://github.com/cmsd2/aximar/pull/66).
11. **GUI events drop under contention.** `app_handle` sits behind a tokio `Mutex` read with `try_lock` in six places (`tauri_output.rs:49,95,161`, `mcp/startup.rs:52,70`, `commands/session.rs:27`); a busy lock skips the emit. `startup.rs:46` spawns a task per effect, so order isn't kept either. The last event of a run is the one most likely missing, and nothing later repairs the GUI. Tracked in [#55](https://github.com/cmsd2/aximar/issues/55). Fixed in [#65](https://github.com/cmsd2/aximar/pull/65).
12. **`list_cells` panics on multi-byte output.** `aximar-mcp/src/server.rs:746` slices `&preview[..200]` by bytes. Tracked in [#49](https://github.com/cmsd2/aximar/issues/49). Fixed in [#62](https://github.com/cmsd2/aximar/pull/62).
13. **Loading a notebook clears its path, then restores it.** `notebookStore.ts:430` clears `filePath` on `notebook_replaced`, which `nb_load_cells` emits; both open paths patch it back (`App.tsx:314`, `useNotebookEvents.ts:125`), so the result depends on event order. Tracked in [#16](https://github.com/cmsd2/aximar/issues/16) and [#40](https://github.com/cmsd2/aximar/issues/40). Fixed in [#73](https://github.com/cmsd2/aximar/pull/73): the path lives in core, and the window follows `path_changed` events.
14. **Core reads `~/.maxima` on Windows.** The copied `maxima_userdir` (`doc_index.rs:551`) dropped mxpm's Windows branch. The LSP uses this copy. Tracked in [#13](https://github.com/cmsd2/aximar/issues/13).
15. **Package symbol conflicts resolve randomly.** `load_runtime_files` iterates a `HashMap` (`doc_index.rs:205`), so when two packages define a symbol the winner changes between runs. Tracked in [#13](https://github.com/cmsd2/aximar/issues/13).

### Robustness (inferred or latent)

16. `try_begin_eval`/`end_eval` are paired by hand in four callers with no drop guard (`session.rs:241-290`); a cancelled request can leave the session Busy for good.
17. `session_ops.rs:57` marks the session Ready before the texput and plotting init runs; init errors are discarded; `ensure_session` checks then starts in two steps, so two callers can both spawn.
18. `inline_images` (`maxima-lsp/src/server.rs:470-505`) joins package-supplied paths with no containment check, so a package's docs can embed any readable local file.
19. `useTheme.ts` writes the default theme on mount, which can race the saved theme read (inferred).
20. Remaining `partial_cmp().unwrap()` at `doc_index.rs:338` and `packages.rs:101,131` (tech-debt #5 / issue #11, partly fixed). Tracked in [#11](https://github.com/cmsd2/aximar/issues/11). Fixed in [#62](https://github.com/cmsd2/aximar/pull/62).

## 2. Cross-cutting themes

**Wire types are copied by hand from Rust to TypeScript, and have drifted.** The cell output shape exists in five TS places; `AppConfig` enums are plain strings. Bug 8 is the visible cost. Generating types with ts-rs or specta would remove the class.

**ipynb conversion exists in two languages.** `aximar-mcp/src/convert.rs` and `src/lib/notebooks-client.ts:42-158` share an algorithm and comments, and already differ (`nbformat_minor` 5 vs 0, `aximar` metadata only in Rust). Tauri adds a third DTO (`LoadCell`). Moving conversion into core and loading by path would remove the TS copy and `nb_load_cells`.

*Status, November 2026:* S1 did this. The `aximar-notebook-format` crate is the one reader and writer for the window, aximar-mcp and batch runs ([#81](https://github.com/cmsd2/aximar/pull/81), [#82](https://github.com/cmsd2/aximar/pull/82)); the window opens and saves by path through Tauri commands, and `convert.rs`, the TypeScript copy and `nb_load_cells` are gone. The same crate, built as WebAssembly, is the VS Code extension's serializer ([#89](https://github.com/cmsd2/aximar/pull/89), cmsd2/maxima-notebook#3).

**Policy lives in the adapters.** The dangerous-call gate appears four times with different rules (`server.rs:297`, `:904`, `batch.rs:94`, Tauri `notebook.rs:333`); run-all twice; input preprocessing three ways (unicode and labels in `evaluation.rs`, labels only in Tauri `evaluate.rs`, unicode only in MCP); catalog search and docs JSON three ways (LSP, MCP, Tauri), with different results.

*Status, November 2026:* `unify-gui-mcp-operations` moved the trust check ([#68](https://github.com/cmsd2/aximar/pull/68)), run-all ([#69](https://github.com/cmsd2/aximar/pull/69)), input preparation ([#70](https://github.com/cmsd2/aximar/pull/70)) and catalog queries ([#74](https://github.com/cmsd2/aximar/pull/74)) into aximar-core, closing [#41](https://github.com/cmsd2/aximar/issues/41). The LSP keeps its own catalog code.

**Maxima text is scanned by hand in at least seven places.** Core's `find_terminators`, `unicode_to_maxima`, `replace_bare_percent`, `has_function_call`, `plotting_lisp_stdin`, plus the LSP's helpers, definition search and folding, and catalog-gen's regex scanner. They disagree on escapes, nested comments and `?`. `maxima-mac-parser` has a real lexer, but core doesn't depend on it.

*Status, November 2026:* label rewriting and Unicode translation now share core's `code_spans`, which knows escapes and nested comments ([#62](https://github.com/cmsd2/aximar/pull/62), [#71](https://github.com/cmsd2/aximar/pull/71)). The other scanners are unchanged.

**Few seams for tests.** `MaximaProcess` is a concrete 1,564-line struct over a live child process, so protocol timeouts, terminator counting and kill paths run only in live tests; `backend.rs`, `overlay.rs`, `drain.rs` and `process.rs` have no unit tests. The frontend has no runner. src-tauri's pure logic (config cascade, payload building, catalog parsing) is untested. `Catalog::load` reads `~/.maxima`, so LSP features have no tests.

**Copies of mxpm code have drifted (issue #13).** Core's doc-index types differ from mxpm's (`body_html` vs `subsections`), `maxima_userdir` lost Windows, and `load_runtime_files` reimplements mxpm's scanner. Beyond #13, catalog-gen's Texinfo pipeline (`parser.rs`, `markdown.rs`, `mapping.rs`, ~2,000 lines, 19 of its 31 tests) has been orphaned since c3de97f: it writes `catalog.json`, which no longer exists, and mxpm does the same job.

**The kernel-events path is off in the shipped app.** Only the `AXIMAR_KERNEL_EVENTS` env var enables it (`process.rs:111`), so overlays always receive empty envelope lists. Board items #22 and #23 cover turning it on.

## 3. Changeability

| Change | Places to edit today |
|---|---|
| New output type (e.g. `image_png`) | ~12: `EvalResult`, `CellOutput`, legacy parser, overlay, `convert.rs` both ways, Tauri `LoadCell`, five TS types, `CellOutput.tsx`, `latex-export` |
| New notebook operation | ~9: two core enums plus three match arms, Tauri command and registration, TS wrapper and two hand-written event types, MCP tool and params |
| New Maxima backend | 7 `match Backend` sites across `backend.rs` and `process.rs` |
| New setting | Rust struct/enum, TS `AppConfig`, `SettingsModal` row and `update()` branch, mirrored branch in `App.tsx` config load |
| New `.mac` item kind | 16 exhaustive matches in 9 files, including the DAP |

## 4. Per-area notes

**Core.** Pure logic is in good shape: parser (42 tests), errors (25), labels (35), the `Session` state machine (20). `protocol.rs` repeats its take/timeout/restore/kill block four times and has two near-identical `evaluate` functions. Backends have no trait. The legacy parser has three safe-path checks that differ only by extension, and its error patterns differ from `debugger::ERROR_MARKERS`. Dead: `read_eval_marker_and_count` (~90 lines), `DocIndexStore::load`, `plotting_lisp_code`. Undo still deep-clones every cell, plot payloads included (`notebook.rs:456`).

**MCP and Tauri.** Both route evaluation, session start and mutation through core, which is the main strength. `server.rs` has grown to 1,488 lines (30 tools); session and notebook tools duplicate each other; `run_all_cells` re-parses `run_cell`'s JSON string (fixed in [#69](https://github.com/cmsd2/aximar/pull/69): both now use core's `run_all`). Tauri maps every core error to `CommunicationError`, so users read "Failed to communicate with Maxima: Cell 'x' not found". `resolve_context` is copied five times; `start_session` and `restart_session` are identical. Tauri `evaluate_expression` has no callers in the frontend.

**Frontend.** The backend owns notebook state and undo, and the store mirrors it: a good spine. Since the March audit, none of its nine refactors landed and the flagged files grew (`App.tsx` 400→526 lines, `SettingsModal` 489→695, `DocsPanel` 317→523, `useNotebookEvents` 157→396). Open, new-tab and refresh flows are copied four to six times. Components call `invoke` directly in several places, and an implicit "active notebook" is read inside three API clients, which causes bug 1. Every keystroke re-renders every cell (subscriptions to the whole cell list). About 10 dead exports, `MathText.tsx` and `ErrorOutput.tsx`; 11 `eslint-disable` comments with no ESLint configured.

**LSP, parser, catalog-gen.** The parser is an outline extractor (four item kinds, no expressions), fault-tolerant and well tested on real files. Its error model is mostly dead: the lexer cannot fail, so five error kinds are never built, and the only remaining diagnostic fires on valid expression statements. The LSP's references, folding and enclosing-call logic use their own scanners instead of the lexer, and match inside strings and comments. `extract_params` is copied verbatim between core and the LSP.

## 5. Status of the earlier audit docs

This doc replaces three audits from March and April 2026: `backend-code-quality.md`, `frontend-separation-of-concerns.md` and `technical-debt.md`. Git history has the originals.

- Resolved and still resolved: duplicated run_cell (now partly regressed by bug 10), lock consolidation, regex statics, blob URLs, session race, undo moved to the backend, label map in the backend.
- Open and worse: MCP `server.rs` size, `config.rs` (624→787 lines), safe-path duplication, protocol pattern, every frontend item.
- Open: undo deep clones, `MultiOutputSink` clone (`output.rs:38`), mixed error types, settings drift (technical-debt #13).
- Partly fixed: NaN panic (technical-debt #5): the cited file moved and three unwraps remain (bug 20).

### Technical-debt items 6, 9 and 10

Issue cmsd2/aximar#14 tracks these three items from the old `technical-debt.md`. All three are still open, and the code now lives in aximar-core.

- **6. No tests for WSL and Docker path translation.** `crates/aximar-core/src/maxima/backend.rs` has no unit tests, although `translate_container_path`, `host_temp_dir` and `decode_wsl_output` are pure.
- **9. The parser doesn't bound LaTeX accumulation.** The legacy parser appends to its multi-line LaTeX buffer with no size limit (`crates/aximar-core/src/maxima/legacy/parser.rs:296-298`), so an unterminated `$$` block grows until the evaluation ends.
- **10. WSL error handling.** The temp-dir `mkdir` result is discarded (`process.rs:389`), `std::fs::copy` from the WSL share has no timeout (`backend.rs:132`), and an `unwrap` remains at `process.rs:314` (safe in practice).

## 6. Existing issues these touch

Issues filed from this audit: [#46](https://github.com/cmsd2/aximar/issues/46) to [#57](https://github.com/cmsd2/aximar/issues/57). Testing the fixes found four label problems, filed as [#58](https://github.com/cmsd2/aximar/issues/58) to [#61](https://github.com/cmsd2/aximar/issues/61).

Earlier issues the audit touches:

- cmsd2/aximar#11 (NaN panic): mostly resolved; three unwraps remain.
- cmsd2/aximar#13 (mxpm copies): confirmed, with drift; catalog-gen's orphaned pipeline is a larger copy.
- cmsd2/aximar#14 (tech-debt 6, 9, 10): all three still open; see [section 5](#technical-debt-items-6-9-and-10).
- cmsd2/aximar#7 (error outputs lost on save and load): related to bug 8, same missing field.
- cmsd2/aximar#22 and #23 (kernel-events): the env-var gate is the current state.

## 7. Strengths worth keeping

- One `evaluate_cell` and `session_ops::ensure_session` serve the GUI, MCP and batch modes.
- All notebook mutation goes through `Notebook::apply` and `CommandEffect`, with undo in the backend.
- The `Session` enum state machine and the `OutputSink` / `EnvelopeObserver` traits.
- MCP tests drive tools without a transport; most run without Maxima.
- The parser's two-phase design (byte spans, then one UTF-16 pass) and its real-file regression test.
- Per-evaluation sentinels, the canonicalised temp-dir check, and the cancel handle held outside the session lock.
