# Core ↔ Tauri Data Transfer

How notebook state crosses the `aximar-core` → Tauri → webview boundary,
what we measured, and which alternatives were tried and set aside.

This exists because the question keeps resurfacing under different
names ("plots feel slow", "why is the payload so big", "can we stream
output?") and each time the same three options get re-derived. Read
this before changing the shape of `notebook-state-changed`.

## Event channels

| Event | Emitted by | Shape |
|---|---|---|
| `notebook-state-changed` | `emit_notebook_state` (`src-tauri/src/commands/notebook.rs`) | **Full snapshot** — every cell, every output |
| `maxima-output` | `TauriOutputSink` (`src-tauri/src/tauri_output.rs`) | Per-line delta, notebook-tagged |
| `maxima-event` | `TauriEnvelopeObserver` (same file) | Per-frame delta, notebook-tagged |
| `app-log` | `emit_app_log` | Per-entry delta |
| `notebook-lifecycle` | `src-tauri/src/mcp/startup.rs` | Single id + verb |

Note the asymmetry: everything except notebook state is already a
narrow per-item event. `notebook-state-changed` is the outlier.

## Current design: full snapshot per command

`emit_notebook_state` fires after **every** notebook command, and
`notebook_state_payload` (`src-tauri/src/mcp/sync.rs`) clones every
cell with its complete output — `plot_svg` as inline SVG text,
`image_png` as base64. There is no delta encoding.

Triggers include:

- the 300 ms debounced input sync while typing (`src/lib/dirty-inputs.ts`)
- every cell status transition (Running → Success / Error)
- add / delete / move / type-toggle, undo / redo, trust changes

On the frontend, `applyBackendState` (`src/store/notebookStore.ts`)
rebuilds every cell object on every event, and `Notebook.tsx` renders
them with a plain `.map()` — no `React.memo`, no virtualization. So
each event also re-renders every cell: KaTeX, Plotly, CodeMirror.

### Why it's shaped this way

The snapshot is **self-healing**. Tauri emits are fire-and-forget
(`let _ = app_handle.emit(...)`) and fan out to every window, so a
dropped or late event costs nothing — the next one repairs state. A
window that mounts halfway through a session converges on its first
event. That property is load-bearing and easy to lose by accident.

## Measurements (2026-07-24)

| Quantity | Value |
|---|---|
| `plot2d(sin(x)/x,[x,-20,20])` gnuplot SVG | 26.6 KB |
| Base64 PNG overhead vs file | ~1.33× |
| Input-sync debounce | 300 ms |

A notebook with ten plots re-ships roughly a quarter-megabyte per
event, including while typing in an unrelated cell.

**Measured verdict: acceptable today.** Interactive performance is
fine at realistic notebook sizes. Nothing below should be implemented
speculatively — implement it when a measurement says to.

## Rejected: file paths + Tauri asset protocol

**What was tried.** `EvalResult` / `CellOutput` carry
`plot_svg_path` / `image_png_path` instead of inline content; the
frontend resolves them with `convertFileSrc()` and lets the webview
load and cache the file. Payload drops from ~26 KB to ~40 bytes per
plot, and `CellOutput.tsx` skips `sanitizeSvg` + `Blob` +
`createObjectURL` entirely.

**Why it was set aside.**

1. It needs three pieces of configuration that are all absent:
   `app.security.assetProtocol` (`enable` + a `scope` allowlist) in
   `tauri.conf.json`; `asset:` / `http://asset.localhost` added to the
   CSP `img-src`, which is currently `'self' blob: data:`; and an
   asset-protocol permission in `src-tauri/capabilities/default.json`.
   Without all three the `<img>` is blocked and the plot renders blank.
2. The scope grants the webview direct read access to a temp
   directory. Today that path is guarded Rust-side by
   `is_safe_svg_path` / `is_safe_image_path`; the asset protocol moves
   part of that trust boundary into config.
3. Measured performance did not justify it.

The plumbing was threaded end-to-end but never populated — the
producer in `envelope/overlay.rs` kept inlining — so it was dead code
in every build it shipped in. It has been removed. **If you re-add it,
do the three config changes in the same commit**, or it will be dead
again.

## Not a data-transfer problem: the UI freeze

The investigation that produced this document started from "the UI
hangs". That turned out to be unrelated: `splitMath` and
`renderMathText` looped forever on text containing an unmatched `$`,
because the plain-text branch's `indexOf("$", i)` returns `i` and
never advances. Eleven strings in `core-doc-index.json` have an odd
`$` count, including the entry for the `$` symbol itself, so hovering
`$` froze the window.

Fixed separately. Recorded here so a future freeze isn't
re-attributed to payload size — payload size causes *slowness*, never
a hard freeze.

## If it does become a problem

### Option 1 — omit outputs when the effect can't have changed them

Pass the `CommandEffect` into `notebook_state_payload` and set
`output: None` for every cell except on output-changing effects; the
frontend keeps its existing output when the field is absent.

Roughly a dozen lines, and it **keeps the self-healing snapshot** —
ids, inputs and status still arrive in full every time. This is the
right first move. Do this before anything more elaborate.

### Option 2 — per-cell deltas keyed on `CommandEffect`

`CommandEffect` (`crates/aximar-core/src/commands.rs`) already names
exactly what changed and carries the `cell_id`; `emit_notebook_state`
receives it and currently ignores it.

| Effect | Payload |
|---|---|
| `CellInputUpdated` | `{ cell_id, input }` |
| `CellStatusUpdated` | `{ cell_id, status }` |
| `CellOutputUpdated` | `{ cell_id, output }` — the one place a large payload is legitimate, once per eval |
| `CellAdded` / `Deleted` / `Moved` / `TypeToggled` | ordered cell-id list (+ the new cell for Added) |
| `NotebookReplaced` / `Undone` / `Redone` | full snapshot — rare, genuinely needs everything |
| `NotebookTrusted` | flag only |

This also lets the store replace a single array entry, so with
`React.memo` on `Cell` only the touched cell re-renders — likely a
bigger real-world win than the bytes saved.

**Prerequisite, not optional:** a monotonic per-notebook `revision:
u64` on every payload. The frontend tracks the last applied revision
and calls `nb_get_state` on a gap. Without it, one dropped event
diverges permanently — you are trading away the self-healing property
described above, and you have to buy it back explicitly.

### Option 3 — asset protocol

See the rejected section. Still viable, still needs its three config
changes.

## Streaming

Streaming incremental cell output is where this stops being
theoretical. A full snapshot per chunk is quadratic in total output
size: emit *n* chunks and you ship the whole notebook *n* times.

**Do not route streamed output through `notebook-state-changed`.** Use
a dedicated append-style event carrying only `{ notebook_id, cell_id,
chunk }`, and let the frontend accumulate. The precedent already
exists in this codebase — `maxima-output` and `maxima-event` are both
per-item deltas that never reuse the snapshot channel, and neither
needs a revision counter because appends are idempotent under
at-most-once delivery in a way that state replacement is not.

The snapshot channel then keeps its current job: whole-notebook truth,
emitted on structural change, cheap to re-apply.

## Invariants worth preserving

- **Self-healing.** Snapshots repair dropped events; deltas need an
  explicit resync path.
- **Multi-window.** Every window receives every event and must be able
  to converge from any starting point.
- **MCP parity.** The MCP server drives the same `Notebook` and the
  same commands; anything added to the GUI event path must not become
  a correctness requirement for MCP-driven mutation.
