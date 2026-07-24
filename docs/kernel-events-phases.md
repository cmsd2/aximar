# kernel-events: phase status

Aximar's migration from scraping Maxima's stdout to consuming
structured envelopes from the [`kernel-events`][ke] package over fd 3.

Until now the phase numbering lived only in commit messages and
scattered code comments, which made "what's left?" unanswerable
without reading the whole branch. This file is the index. Update it
when a phase lands.

[ke]: https://github.com/cmsd2/maxima-kernel-events

## The two paths

Aximar runs one of two pipelines per session:

- **Legacy** — stdout scraping. `tex(%)` for LaTeX, a
  `__AXIMAR_LABEL__` print for the output label, a per-eval unique
  sentinel print as the read terminator, regexes over the resulting
  text. Lives in `crates/aximar-core/src/maxima/legacy/`.
- **Wired** — fd-3 envelopes. Termination from `eval_end` counts,
  content from `eval_result` / `output` / `display` / `error` / `vars`.
  Lives in `crates/aximar-core/src/maxima/envelope/`.

Selection is `MaximaProcess::has_events_channel()`, which is true only
when **all** of these hold: Unix, a backend that inherits fds (not
Docker / WSL / SSH), and `AXIMAR_KERNEL_EVENTS` set truthy.

**The wired path is opt-in and off by default.** Everything below in
"Landed" is dark code in a default build.

## Landed

| Phase | Commit | What |
|---|---|---|
| A | `8a92316` | fd-3 pipe + envelope reader, parallel to the legacy parser |
| A.1 | `679a042`, `f2cb8af` | Drain envelopes alongside the legacy sentinel; drop per-envelope logging, drain the init backlog |
| A.2 | `6f48929` | Terminate the eval on `eval_end` count instead of a stdout sentinel |
| — | `d1ba720`, `16aa494`, `1356d40` | Consolidation: replace the 50 ms grace-period drain with a stdout+envelope fence, then go single-source (background drain owns stdout, eval reads only envelopes), then drop the now-unneeded housekeeping prints |
| A.3 | `da9b350` | Build `EvalResult` on the wired path without calling the legacy parser at all |
| B | `5b3d228` | `error` envelopes feed `EvalResult.error` directly |
| B.1 | `a4919fc` | `cancelled` error kind maps to `AppError::EvalCancelled` |
| B.2 | `e29f422` | Drain envelopes inside internal-protocol commands |
| B.3 | `ce0c8c0` | `vars` envelope replaces the `__AXIMAR_VARS__` scrape |
| B.4 | `9c92d56` | `eval_result` envelope feeds latex + output label |
| B.5 | `7565d38` | SVG plots arrive via `eval_result` |
| B.6 | `9b79087` | PNG / JPG arrive via `eval_result` |
| C | `f10d577` | `display` envelopes feed `EvalResult.plot_data` |
| D | `ec2ed4a` | Cooperative cancellation over the fd-4 transport |
| D.1 | `3b1255c` | Cancel exposed through Tauri + a per-cell stop button |
| — | `4f5b55b` | `EnvelopeObserver` tee → GUI "Events" log tab |

## Coverage

`crates/aximar-core/tests/events_smoke.rs` — 12 tests against a real
Maxima spawned with `AXIMAR_KERNEL_EVENTS=1`.

**These are not `#[ignore]`d.** They are gated at runtime by
`AXIMAR_RUN_LIVE_TESTS`, and each one returns early when it isn't set.
So a plain `cargo test` reports "12 passed" having executed none of
them, and `-- --ignored` runs zero. Neither tells you anything. The
only invocation that exercises the wired path is:

```
AXIMAR_RUN_LIVE_TESTS=1 cargo test -p aximar-core --test events_smoke
```

Treat a green `cargo test --workspace` as saying nothing whatsoever
about kernel-events.

They cover: envelope drain alongside the legacy sentinel, error →
`EvalResult`, cancelled → `EvalCancelled`, internal-command envelopes
not leaking into the next eval, init-drain ordering, display →
`plot_data`, fd-4 cancel aborting a long eval, vars, latex + label,
and PNG arrival.

Not covered: anything under "Remaining".

## Remaining

The package defines **18 envelope types**
(`kernel-events/schemas/envelopes/v1/` — 20 files, of which
`README.md` and `common.json` are not types). Aximar's `Envelope` enum
models 12 of them and acts on **6**: `eval_end`, `error`,
`eval_result`, `output`, `display`, `vars`.

### Next: flip the default

`kernel_events_enabled()` in `maxima/process.rs` still carries:

> *"Off by default during Phase-A so the legacy pipeline keeps owning
> behaviour until the envelope path is consuming envelopes for real."*

That condition is met — the comment is stale. Flipping it is the
gating step; until then none of the above ships to users.

Two things to settle first:

1. **Docker / WSL / SSH can't inherit fd 3.** Those backends stay
   legacy regardless, so flipping the default does not retire the
   legacy path — it makes the dual-path maintenance burden permanent
   unless those backends get a different transport (a socket, or
   envelopes multiplexed over stdout with a framing marker).
2. **Coverage is thin for a default, and quiet about it.** 12 smoke
   tests plus the 35 Maxima-backed `ax_plotting` tests are the whole
   automated story — and the smoke tests pass vacuously unless
   `AXIMAR_RUN_LIVE_TESTS` is set, so a green CI run is not evidence
   the wired path works. Wire that env var into CI before making this
   the default, and exercise plots, errors, cancellation and variables
   by hand.

Suggested shape: a settings toggle rather than a bare env var, so the
path can be switched without restarting from a shell.

### Modeled but inert

Parsed into an `Envelope` variant, then ignored. Each one that gets
consumed deletes a fragile stdout heuristic:

| Kind | Replaces |
|---|---|
| `stdin_request` | `ASSUMPTION_QUESTION_RE` — a regex hunting stdout for "Is x positive?" and friends, then guessing an answer |
| `debug_enter` / `debug_leave` | `dbm:N>` prompt scraping in `maxima/debugger.rs` (`PromptKind`) |
| `capabilities` / `ready` | The `__AXIMAR_READY__` / `__AXIMAR_INIT_DONE__` init sentinels |
| `eval_begin` | Nothing today — would supply per-eval correlation ids, currently inferred positionally |

The `stdin_request` and debugger ones are the highest value: both
replace pattern-matching on human-readable prose with a typed signal,
and both currently fail silently when Maxima's wording changes.

### Not modeled at all

These hit the `#[serde(other)]` catch-all and are dropped on the
floor. The kernel emits them today; aximar has never seen them:

`stream_begin`, `frame`, `progress`, `stream_end`, `stream_error`,
`log`

That is a complete streaming protocol from the kernel's
`stream-events.lisp` — incremental output, progress reporting, and
frame delivery — with no host-side half. If streaming is wanted, the
transport already exists and the work is entirely in aximar.

**Before building it, read
[`core-tauri-data-transfer.md`](core-tauri-data-transfer.md).**
Streamed output must not travel on `notebook-state-changed`: that
channel emits a full notebook snapshot per event, so *n* chunks would
ship the notebook *n* times. It needs an append-style per-cell event,
in the shape `maxima-output` and `maxima-event` already use.

## Suggested order

1. Flip the default (behind a settings toggle), after resolving the
   Docker/WSL question and doing a manual pass.
2. `stdin_request`, then `debug_enter` / `debug_leave` — each deletes
   a heuristic.
3. Streaming, as its own design pass with a transport decision first.

Nothing here is urgent. The wired path is complete for ordinary
evaluation; what remains is reach (backends), robustness (replacing
heuristics), and new capability (streaming).
