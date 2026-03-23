# Technical Debt & Areas Needing Attention

Audit performed 2026-03-23 against `master` at commit `98277a8`.

## Critical

### ~~1. Regex objects created per-call in parser.rs~~ — RESOLVED

Already fixed: `LABEL_RE` and `SVG_PATH_RE` are `LazyLock` statics (parser.rs lines 11–15). The local bindings on lines 77 and 214 just borrow the statics.

### ~~2. Blob URL memory leak in CellOutput.tsx~~ — RESOLVED

Moved blob URL revocation from synchronous render-time ref tracking to a `useEffect` cleanup that depends on `plotBlobUrl`. The URL is now revoked both when it changes and on final unmount.

### ~~3. Silent error swallowing~~ — RESOLVED

All `.catch(() => {})` handlers in `SettingsModal.tsx`, `CommandPalette.tsx`, and `App.tsx` now log errors via `useLogStore.addEntry` so failures appear in the log panel.

## High

### ~~4. Process/status race condition in session management~~ — RESOLVED

Combined status and process into a unified `Session` state machine (`session.rs`) behind a single `Mutex`. An `AtomicU8` mirror provides lock-free status reads for the UI. `SessionStatus::Busy` is now set during evaluations via `try_begin_eval()`/`end_eval()` transitions.

### 5. Panic on NaN in catalog search scoring

**File:** `src-tauri/src/catalog/search.rs` (lines ~46, 72)

`partial_cmp().unwrap()` on f64 scores will panic if a score is NaN. Replace with `.unwrap_or(std::cmp::Ordering::Equal)`.

### 6. No tests for WSL/Docker backend path translation

**File:** `src-tauri/src/maxima/backend.rs` (lines ~46–100)

WSL UNC path construction and Docker host temp directory logic have no test coverage. This is complex cross-platform code that's difficult to test manually.

## Medium

### 7. Undo/redo snapshot timing gap

**File:** `src/store/notebookStore.ts` (lines ~71–192)

`updateCellInput` uses a 500ms debounce for snapshots. If a user types rapidly then immediately hits Ctrl+Z, the last keystrokes may not be in the undo history. The pending snapshot should be flushed when undo/redo is triggered.

### 8. buildLabelLatexMap rebuilds on every cell output

**File:** `src/hooks/useMaxima.ts` (line ~145)

`buildLabelLatexMap()` does an O(n) scan of all cells on every evaluation result. Fine for small notebooks but will scale poorly. Consider caching or incremental updates.

### 9. Parser doesn't bound LaTeX accumulation

**File:** `src-tauri/src/maxima/parser.rs` (lines ~140–153)

Multi-line LaTeX accumulation has no upper bound. If Maxima crashes mid-output or produces pathological output, the buffer grows without limit. Add a reasonable cap.

### 10. WSL error handling gaps

**File:** `src-tauri/src/maxima/process.rs` (lines ~125–165)

- `wsl mkdir -p` for temp directory creation has no error handling if it fails
- `fs::copy()` from WSL has no timeout — could hang if WSL is unresponsive
- `host_temp_dir().unwrap()` in process.rs will panic if the path is missing

## Low

### ~~11. Repeated regex compilation in errors.rs~~ — RESOLVED

Already fixed: all regexes in `errors.rs` (`ARG_COUNT_RE`, `UNDEFINED_VAR_RE`, `UNDEFINED_FUNC_RE_1`, `UNDEFINED_FUNC_RE_2`, `MISSING_ASSUMPTION_RE`, `LOAD_FAILED_RE`) are `LazyLock` statics.

### 12. CodeMirror state coupling

**File:** `src/hooks/useCodeMirrorEditor.ts`

Three levels of state (CM editor state, Zustand store, external cell input) are synchronized via an `isInternalUpdate` flag that isn't async-safe. The filtered keymap is also re-computed on every render. Not currently causing bugs but fragile.

### 13. Settings modal state drift

**File:** `src/components/SettingsModal.tsx`

`setConfig()` is fire-and-forget with no verification the backend received it. If the IPC call fails silently (see item 3), the frontend shows updated settings but the backend retains the old values. On restart, old settings reappear.

## Test Coverage Notes

- **errors.rs**: 23 tests covering all 12 error patterns (division by zero, syntax errors, arg count, undefined variable/function, lisp error, divergent integral, inconsistent equations, assumption required, matrix dimensions, premature termination, package not found).
- **parser.rs**: 22 tests covering basic parsing, junk LaTeX filtering (7 patterns), error context accumulation, label edge cases, multi-line LaTeX, sentinel filtering, and empty/minimal input.
- **backend.rs**: WSL/Docker path translation still has no test coverage (item 6).
