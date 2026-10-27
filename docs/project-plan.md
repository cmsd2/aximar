# Aximar project plan

Written 2026-10-27, from a review of aximar, ax-plots, mxpm, kernel-events and the VS Code extension, and updated against v0.15.0. Status lives in the [Maxima ecosystem plan project](https://github.com/users/cmsd2/projects/2): one item per fix and per stream deliverable, with Stream, Phase and Kind fields. This doc holds the reasoning.

Companion docs: [kernel-events-phases.md](kernel-events-phases.md) (wired-path status), [technical-debt.md](technical-debt.md) (older audit), [ide-features.md](ide-features.md) (GUI vs editor split), and ax-plots' `doc/roadmap.md` § Packaging (the original packaging proposal, 2026-10-19).

## Where things stand

- The core works. `cargo test --workspace` passes (501 tests at v0.15.0), and kernel-events phases A through D have landed on master.
- The kernel-events wired path is still opt-in (`AXIMAR_KERNEL_EVENTS`). Its 12 smoke tests pass without running unless `AXIMAR_RUN_LIVE_TESTS=1` is set.
- aximar bundles a copy of ax-plots ([ax_plotting.mac](../crates/aximar-core/src/maxima/ax_plotting.mac)). v0.15.0 re-synced it to ax-plots 0.2.0 with two local changes: no `load(...)` lines, and `ax__deliver_plotly`, which sends plots through kernel-events' `emit_display` when that is loaded. kernel-events, by contrast, loads from `~/.maxima` and must be installed with mxpm.
- The VS Code extension and Aximar both read and write `.macnb`, but they disagree on outputs, cell IDs and metadata. An Aximar notebook opened and saved in VS Code shows raw SVG, Plotly JSON or PNG base64 as text when reopened in Aximar.
- Reactive plot controls work only in the Aximar GUI. Their host half is a Tauri command ([reactive.rs](../src-tauri/src/commands/reactive.rs)), so MCP, VS Code and any future kernel can't reach it.
- Aximar has no Jupyter kernel, so Quarto can't execute Maxima. Quarto renders pre-executed Aximar notebooks, but shows its "Unable to display output" placeholder for every LaTeX and Plotly output.

## Decisions

| Decision | Reason |
|---|---|
| Aximar stays a notebook. Editing, debugging and package authoring stay in VS Code. | Building an IDE in Aximar would duplicate the extension. `ide-features.md` already lists a full editor as a non-goal. |
| Every front end (GUI, VS Code, MCP, Jupyter/Quarto) gets as much of the ecosystem as possible. | Aximar's value is in features other Maxima front ends lack. |
| Each feature has three layers: a Maxima package, an aximar-core API, and a shared renderer. Hosts are thin adapters. | A feature written once then reaches every host. |
| Packages install through the mxpm library, after the user agrees, at the exact versions each Aximar release was tested with, and not from a bundled copy. | Built-in examples keep working with one click. Nothing writes to `~/.maxima` without consent. Package fixes reach users without an Aximar release. |
| aximar stays one repo. | Every Rust crate depends on aximar-core by path. Planned work changes core and hosts together. |
| vscode-extension stays a separate repo. Shared TypeScript ships as npm packages built in an npm workspace inside aximar. | Keeps the extension's release flow independent. |
| The Jupyter kernel builds on aximar-core. maxima-jupyter is a reference only. | maxima-jupyter runs inside Maxima's Lisp. It can load Maxima packages, but none of aximar-core's host-side features. |

## Fixes

Bugs and debt found in the review. A fix that a stream replaces properly has its stream in the "Lands in" column. Patch those now only if the stream is far off.

| ID | Problem | Where | Lands in |
|---|---|---|---|
| F1 ([#4](https://github.com/cmsd2/aximar/issues/4)) | When the events fd is wired but kernel-events fails to load, every cell runs to the eval timeout. `has_events_channel()` doesn't check that the package loaded. Wait for the `ready`/`capabilities` envelope at init, and fall back to legacy if it never arrives. Found by reading the code, not reproduced. | `maxima/process.rs` | Phase 0 |
| F2 ([#5](https://github.com/cmsd2/aximar/issues/5)) | The kernel-events init `:lisp` line uses `sb-sys:`/`sb-ext:`, which fails at read time on Maxima builds that don't use SBCL. `handler-case` can't catch a read-time error. | `maxima/process.rs` `events_init_snippet` | Phase 0 |
| F3 ([#6](https://github.com/cmsd2/aximar/issues/6)) | Aximar treats a `text/plain` fallback as real output, so plots saved in VS Code show their source as text. | `src/lib/notebooks-client.ts:143`, `aximar-mcp/src/convert.rs` | S1 (patch in Phase 0) |
| F4 ([#7](https://github.com/cmsd2/aximar/issues/7)) | `error` outputs are ignored on read, and Aximar never saves errors at all. | both readers and writers | S1 |
| F5 ([#8](https://github.com/cmsd2/aximar/issues/8)) | Outputs loaded from a file look current, but their Maxima labels are gone. `%oN` for a loaded cell goes to Maxima unchanged and means something else. VS Code restarts numbering at 1 on each new kernel. | `maxima/labels.rs` `rewrite_labels`, `notebook.rs` LoadCells, extension `controller.ts` | S1 |
| F6 ([#9](https://github.com/cmsd2/aximar/issues/9)) | The GUI save writes only `kernelspec` and empty cell metadata. The Rust `NotebookMetadata` drops unknown keys on read. | `notebooks-client.ts` `buildNotebook`, `notebooks/types.rs` | S1 |
| F7 ([#10](https://github.com/cmsd2/aximar/issues/10)) | nbformat 4.5 requires a cell `id`. aximar-mcp and VS Code both declare 4.5 without writing one. The GUI writes 4.0. No reader keeps IDs. | `convert.rs`, extension `serializer.ts`, `notebooks-client.ts` | S1 |
| F8 ([#11](https://github.com/cmsd2/aximar/issues/11)) | `partial_cmp().unwrap()` panics on a NaN score. This is the old tech-debt item 5, now in new files. | `catalog/packages.rs:101,131`, `catalog/doc_index.rs:338` | Phase 0 |
| F9 ([ax-plots#9](https://github.com/cmsd2/ax-plots/issues/9)) | `ax__deliver_plotly` exists only in the bundled copy. Upstream ax-plots always writes a temp file and prints its path, so loading upstream ax-plots into a kernel-events session leaks the path into the cell's text output. Move `ax__deliver_plotly` into ax-plots so the bundle's one remaining difference is the missing `load(...)` lines. | ax-plots `ax-plots.mac` | Phase 0 |
| F10 ([#12](https://github.com/cmsd2/aximar/issues/12)) | The kernel-events smoke tests pass without running in CI. Install Maxima and kernel-events in CI and set `AXIMAR_RUN_LIVE_TESTS=1`. | `.github/workflows/build.yml` | Phase 0 |
| F11 ([#13](https://github.com/cmsd2/aximar/issues/13)) | aximar-core copies the mxpm doc-index types and the user-dir lookup instead of using `mxpm-doc-index` 0.1.1 and `mxpm-core` 0.2.0 from crates.io. | `catalog/doc_index.rs` | Phase 0 |
| F12 (project draft) | The extension's `docs/architecture.md` says it starts aximar-mcp with `--no-auth`. The code uses `--allow-dangerous` and a bearer token. | vscode-extension | Phase 0 |
| F13 (project draft) | The workspace's ax-plots checkout is 30 commits behind `origin/master` and pre-0.2.0. It also has one unpushed commit (`6497f60`, an earlier version of the `emit_display` delivery) that `ax__deliver_plotly` replaces. `~/.maxima/ax-plots` symlinks to this checkout, so `load("ax-plots")` on the development machine loads stale code. Drop `6497f60` (or fold anything still useful into F9) and fast-forward. Local environment only, but it affects any S2 testing that uses the editable install. | `maxima-workspace/ax-plots` | Phase 0 |

[technical-debt.md](technical-debt.md) items 6, 9 and 10 (WSL/Docker path tests, LaTeX accumulation bound, WSL error handling) are still open and fit in Phase 0 too ([#14](https://github.com/cmsd2/aximar/issues/14)).

## Streams

### S1 Notebook format

Issues: [#15](https://github.com/cmsd2/aximar/issues/15), [#16](https://github.com/cmsd2/aximar/issues/16), [#17](https://github.com/cmsd2/aximar/issues/17).

Goal: every host reads and writes `.macnb` identically, and nothing gets lost in a round trip.

- Write `docs/notebook-format.md`, covering:
  - nbformat 4.5 with stable cell `id`s;
  - the `metadata.aximar` keys, including `requires`;
  - one bundle per output, with a preferred-MIME order for reading;
  - `stream` for print output, `execute_result` for the final value, `display_data` for plots, `error` for errors;
  - how output labels persist and when loaded outputs count as stale.
- Make one Rust serializer in aximar-core. The GUI saves and opens through Tauri commands instead of its own TypeScript copy, and aximar-mcp uses it too.
- Publish an npm package (working name `notebook-format`) for the extension, with the same rules.
- Ship shared test notebooks with the npm package. The Rust tests and the extension's tests both run against them.
- Mark loaded outputs as stale in both apps. A display label (`%oN`) that maps to no live output becomes an error ("re-run cell N") instead of passing through.

Fixes F3 to F7.

### S2 Packages

Issues: [#18](https://github.com/cmsd2/aximar/issues/18), [maxima-mxpm#3](https://github.com/cmsd2/maxima-mxpm/issues/3), [#19](https://github.com/cmsd2/aximar/issues/19), [#20](https://github.com/cmsd2/aximar/issues/20), [#21](https://github.com/cmsd2/aximar/issues/21).

Goal: Aximar installs ax-plots and kernel-events as ordinary mxpm packages, at the versions in its package BOM, after the user agrees, from every host. The bundled copy goes away.

This stream carries out the proposal in ax-plots' `doc/roadmap.md` § Packaging (2026-10-19). That doc's sequencing says the release after v0.15.0 starts with a design doc in aximar covering its questions. So S2's first deliverable is `docs/package-management.md`, which settles the questions below. Where this plan already takes a position, it's stated.

**Design questions (from the ax-plots roadmap):**

1. **Version pinning.** The index pins one commit per package, with no ranges. kernel-events' envelope schema is a contract with Aximar's parser, so an index update can break an installed Aximar. Position: each Aximar release carries a **package BOM** (bill of materials): the exact package versions, with hashes, that the release ships and was tested with. Aximar installs those and offers newer ones as an explicit update. It's a BOM rather than a lockfile because nothing is resolved: it records what a release contains, as a distribution's manifest does. This replaces the "minimum version" check in the first draft of this plan.
2. **Offline first run.** Position: ship the archives the BOM lists inside the app and install them through mxpm's local-path install (`install_local_package`). The consent prompt then works without a network, and the packages are still ordinary installs.
3. **Install location.** In the shared `~/.maxima`, a developer's editable install wins. That's useful for development, but risky for users with a half-broken install. F13 shows the risk on this machine: the editable ax-plots there is pre-0.2.0. An Aximar-private directory avoids conflicts but duplicates packages. Position (decided 2026-10-27):
   - **Aximar's own dependencies** (ax-plots, kernel-events) install at their BOM versions into an Aximar-owned folder. Each session searches that folder first, ahead of `~/.maxima`.
   - **Everything else** the user installs (numerics, for example) goes in `~/.maxima` as now, and stays visible in Aximar sessions. The user's `maxima-init.mac` still runs.
   - **Package developers** get a setting to let `~/.maxima` win, so an editable checkout can be tested in Aximar.
   - **The BOM format** is mxpm's, not Aximar's, and mxpm can install a BOM into any folder its caller names. Other projects can then use an Aximar release's BOM as their set of versions ("the versions Aximar 0.16 ships"). A project lockfile, if per-project environments ever arrive, would be a separate file produced by resolving a project's own dependencies (see "Not planned").
4. **Loading.** Replace session-init injection with `load("ax-plots")` / `load("kernel-events")` at startup, so examples still need no `load` cell. Every aximar-core host (GUI, aximar-mcp for VS Code, aximar-kernel) shares the init, so this covers all of them. An explicit `load` cell is only needed for notebooks meant to run in plain Maxima. The first draft of this plan added one to every example; that's dropped.
5. **Docker and WSL backends.** Packages must exist inside the container or WSL filesystem, so the install target is per backend. The package service takes the backend as an input.
6. **What packaging doesn't fix.** kernel-events' text-output capture is SBCL-only, and the fd-3 transport is Unix plus Local backend only. Both the envelope path and the stdout parser remain. S3 covers the transport that could unify them.
7. **mxpm's library API.** mxpm's library surface wasn't designed as a stable API. Installation probably moves into `mxpm-core` with a deliberate interface (mxpm repo). The work this review found:
   - replace the `eprintln!`/indicatif output in `install.rs` with a progress callback;
   - move the synchronous `git2` clone into `spawn_blocking`;
   - fall back to the cached index when offline;
   - keep the CLI-only dependencies (clap, dialoguer, tera, …) out of the library crate;
   - add an upgrade operation (today `AlreadyInstalled` forces remove then install);
   - install a BOM (exact versions and hashes) into a named folder, from the index or from local archives, as first-class operations;
   - consider listing ax-plots as a release tarball, so aximar can build mxpm without git2 and OpenSSL.

**Added by this plan:**

- **Consent.** The roadmap doesn't cover asking the user. The first install, and every update beyond the BOM, goes through a prompt in each host:
  - Aximar: a banner in the style of `TrustBanner`;
  - VS Code: a notification;
  - MCP: an `install_package` tool the client approves;
  - Jupyter kernel: a yes/no `input_request`.
- **An aximar-core package service.** It works out what a notebook needs (from `metadata.aximar.requires`, or from an undefined-function error looked up in the function-to-package map), checks what's installed against the BOM for the session's backend, and installs or updates after consent. `requires` metadata depends on S1, because the GUI save drops metadata today.
- **kernel-events** installs through the same prompt, offered from the setting that turns on the wired path. With F1 in place, a missing install falls back to the legacy path.
- **Removing the bundle:** `ax_plotting.mac`, `ax_plotting.lisp`, the stdin `:lisp (progn …)` trick, and the embedded catalog and docs JSON. Do this only after VS Code and MCP can prompt (Phase 2), because aximar-mcp uses the same copy.

Maxima picks up a package installed mid-session without a restart, because `~/.maxima/**` is searched each time `load` runs (tested on 5.50). The Aximar-owned folder (question 3) goes at the front of `file_search_maxima` and `file_search_lisp` at session start, rather than replacing the user directory through `MAXIMA_USERDIR`, which would hide the user's other packages.

### S3 Kernel protocol

Issues: [#22](https://github.com/cmsd2/aximar/issues/22), [#23](https://github.com/cmsd2/aximar/issues/23).

Goal: the kernel-events wired path becomes the default on local Unix sessions.

- Prerequisites: F1, F2, F10.
- Replace the environment variable with a setting, then flip the default.
- Act on the envelopes that are parsed but ignored:
  - `stdin_request` replaces the `ASSUMPTION_QUESTION_RE` regex;
  - `debug_enter`/`debug_leave` replace prompt scraping;
  - `ready`/`capabilities` replace the init sentinels.
- Remaining work stays as listed in [kernel-events-phases.md](kernel-events-phases.md): streaming envelopes, and Docker/WSL transports (deferred).
- Longer term: "Option C" in kernel-events-phases.md (framed lines multiplexed over stdout) would work on every backend and platform, including Windows. It's the only route to retiring the legacy stdout parser, and it would remove the per-backend split that S2 question 6 describes.

### S4 Renderers and reactive parity

Issues: [#24](https://github.com/cmsd2/aximar/issues/24), [#25](https://github.com/cmsd2/aximar/issues/25), [#26](https://github.com/cmsd2/aximar/issues/26).

Goal: one renderer for LaTeX, Plotly (including `animate()` frames), reactive controls and errors, used by every front end.

- Move replot out of `src-tauri` into an aximar-core function, keeping the name and view-id validation. Expose it as a Tauri command, an MCP tool and a Jupyter comm.
- Publish a renderer npm package (working name `renderers`), starting from the VS Code extension's renderer, which already handles frames. The renderer receives a replot function from its host:

  | Host | Replot path |
  |---|---|
  | Aximar | Tauri `invoke` |
  | VS Code | renderer `postMessage` → extension → MCP tool |
  | JupyterLab | comm `aximar.reactive` |
  | Static HTML (Quarto, nbconvert, GitHub) | none: controls disabled, or slider positions computed ahead as Plotly frames |

- Build a standalone bundle for `text/html` outputs, so static renderers show interactive Plotly.
- The Aximar frontend switches to the package.

### S5 Jupyter and Quarto

Issues: [#27](https://github.com/cmsd2/aximar/issues/27), [#28](https://github.com/cmsd2/aximar/issues/28), [#29](https://github.com/cmsd2/aximar/issues/29).

Goal: JupyterLab, `nbconvert --execute` and Quarto `.qmd` files run Maxima with the aximar features.

- Add `crates/aximar-kernel`. It handles one session per process through `aximar_core::session_ops`, with the same init as the GUI and MCP. It supports execute, complete and inspect (from the core catalog), interrupt through the cancel handle, and an `install` subcommand that writes the kernelspec.
- Output bundles: LaTeX in `text/latex` wrapped in `$$…$$`; Plotly in raw `application/x-maxima-plotly` plus `text/html` with the S4 bundle. Quarto 1.10 renders both forms (tested); it renders neither the raw Plotly MIME type nor `application/vnd.plotly.v1+json`.
- Add `quarto_setup_cell` support for document options and daemon mode ([Quarto docs](https://quarto.org/docs/advanced/jupyter/kernel-execution.html)).
- Add a JupyterLab MIME renderer extension using the S4 package, with reactive controls over the comm.
- Later: Jupyter's debug protocol on top of maxima-dap.
- Use `nbconvert --execute` to run `demos/notebooks/*.macnb` in CI.

### S6 VS Code extension parity

Items: four drafts in the project, because cmsd2/maxima-notebook has issues turned off.

Goal: the extension gets the features that are currently Aximar-only.

- Use the `notebook-format` and `renderers` packages, dropping its own copies.
- Reactive controls through renderer messaging.
- The package install notification (S2).
- Drop its copy of the label-rewriting logic once core handles labels per session.
- Add basic CI. The extension has none today.

### S7 Aximar GUI

Issues: [#30](https://github.com/cmsd2/aximar/issues/30), [#31](https://github.com/cmsd2/aximar/issues/31), [#32](https://github.com/cmsd2/aximar/issues/32).

Goal: a better notebook, not an IDE.

- A Packages panel (browse the index, install, remove, show which copy is active), built on S2.
- maxima-lsp completions in cells, replacing the separate CodeMirror catalog where they overlap.
- "Open in VS Code" for `.mac` files and notebooks.
- A way to show stale outputs (S1).

## Phases

Each phase lists the work from each stream. A phase ends when its exit check passes.

### Phase 0: Fixes

F1, F2, F8, F9, F10, F11, F12, F13, the F3 patch if S1 is more than a few weeks away, and the open technical-debt items.

Exit: CI runs the live kernel-events tests. Upstream ax-plots and the bundled copy differ only by the `load(...)` lines. The workspace ax-plots checkout matches `origin/master`.

### Phase 1: Foundations

- S1: format spec, one core serializer, test notebooks, npm workspace in aximar, `notebook-format` package.
- S2: `docs/package-management.md` answering the seven roadmap questions. mxpm library work (mxpm repo), shaped by those answers.
- S4: replot moved into core.
- S3: wired-path setting.

Exit: an Aximar → VS Code → Aximar round trip of every demo notebook loses nothing. Replot works through MCP.

### Phase 2: Packages and protocol

- S2: package BOM and bundled package archives, package service with per-backend install targets, prompts in Aximar, MCP and VS Code, startup `load` replacing init injection, bundled copy removed.
- S3: wired path on by default, with `stdin_request` and `ready` in use.
- S4: `renderers` package published, Aximar frontend switched over.
- S6: extension uses both npm packages and shows install prompts.

Exit: a fresh install with no `~/.maxima` packages and no network opens a plotting example, asks once, installs the BOM's ax-plots and runs it, in both Aximar and VS Code.

### Phase 3: New front ends

- S5: aximar-kernel, Quarto rendering, CI runs the demos.
- S4/S5: JupyterLab renderer extension with reactive controls over the comm.
- S6: reactive controls in VS Code.

Exit: a `.qmd` using `ax_plot2d` renders interactive HTML with Quarto. A reactive example works in Aximar, VS Code and JupyterLab.

### Phase 4: Polish

S7 (Packages panel, LSP completions, Open in VS Code), Quarto setup cell and daemon mode, Jupyter debug protocol, frame precomputation for reactive controls in static output.

## Dependencies

| Item | Needs |
|---|---|
| S3 default flip | F1, F2, F10 |
| S2 implementation | S2 design doc, mxpm BOM installs from the index and from local archives |
| S2 removing the bundled copy | S2 prompts in Aximar, VS Code and MCP |
| S2 `requires` metadata | S1 (the GUI save drops metadata today) |
| S4 renderer in VS Code | S4 package published |
| S5 Plotly in Quarto | S4 standalone bundle |
| S5 reactive in JupyterLab | S4 replot in core, JupyterLab extension |
| S6 | S1 and S4 packages |

## Open questions

- npm scope for the packages: `@aximar/*` needs an npm org of that name, otherwise use a user scope.
- Reactive controls in static output: computing frames ahead multiplies with each control. Decide on a limit, or disable controls instead.
- "Frozen" outputs: S1 treats this as stale outputs whose labels no longer exist. Confirm nothing else was meant.

## Not planned

- Turning Aximar into an IDE (file tree, editor, debugger UI).
- Merging vscode-extension into this repo.
- Building on maxima-jupyter.
- Docker and WSL kernel-events transports (deferred in kernel-events-phases.md).
- Per-project Maxima environments for users: `mxpm.toml` + `mxpm.lock` + a project `.maxima/` used as `MAXIMA_USERDIR`, with `mxpm sync` / `mxpm run` and every tool detecting the project. The mechanism exists: `MAXIMA_USERDIR` isolates packages (tested on 5.50), and mxpm and aximar-core both honour it. It isn't worth the cost yet:
  - the ecosystem has about six packages and no dependency resolution;
  - every front end would have to detect environments;
  - Quicklisp and SBCL's compile cache stay shared, so the isolation leaks.

  Revisit when two projects on one machine need different versions of the same package, or when frozen notes (such as Study's) render differently because a package changed. Until then, a project can record `mxpm list` output alongside its other locks.
