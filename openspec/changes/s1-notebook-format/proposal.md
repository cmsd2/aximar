## Why

A `.macnb` notebook is read and written by three implementations that disagree: a TypeScript reader and writer in the desktop app (`src/lib/notebooks-client.ts`), a Rust converter in aximar-mcp (`convert.rs`), and the VS Code extension's serializer. Round trips lose work:

- Error outputs are never written, and the `error` outputs the extension writes are skipped, so a failed cell reopens empty (#7).
- The app writes only `kernelspec` and empty cell metadata, and the Rust reader drops unknown keys, so metadata other tools add is lost (#9). S2's `metadata.aximar.requires` depends on this.
- No reader keeps cell IDs, and no writer writes them though two claim nbformat 4.5, so nothing keyed on a cell survives reopening (#10).
- The extension adds a `text/plain` copy to every output, which both Aximar readers show as text, so plots saved in VS Code show their source (#6).
- Friendly error explanations are computed but never stored on a cell output, so they never reach the window (#56).

Label numbering differs too. Aximar continues numbering after a loaded notebook's highest count; the extension restarts at 1 each kernel; loaded outputs look current in both (#8). And since #70, `evaluate_expression` resolves gutter labels on the server, while the extension resolves them itself and sends Maxima's real labels, which the server now rejects: label references in VS Code notebooks fail.

The project plan's S1 stream (#15, #16, #17) is the fix: one written format, one serializer, and shared test notebooks both apps pass.

## What Changes

- **A format spec**, `docs/notebook-format.md`, with the requirements in a new `notebook-format` capability: nbformat 4.5 with stable cell IDs; unknown notebook and cell metadata kept; the `metadata.aximar` keys; which output types hold what (`stream`, `execute_result`, `display_data`, `error`); the MIME preference for reading a bundle, with `text/plain` ignored when a richer type is present.
- **One serializer in aximar-core** (`notebooks::format`), used by the desktop app, aximar-mcp and batch runs. The window opens and saves through new Tauri commands that read and write the file in the backend; the TypeScript serializer, `nb_load_cells` and aximar-mcp's `convert.rs` go.
- **Cells keep their metadata and IDs** in the core model, raw cells are kept (read-only), and outputs keep their errors and friendly explanations.
- **Results carry a plain-text form.** Evaluation captures Maxima's one-line form of each final value (`string(%)`) next to its LaTeX, on both the legacy and kernel-events paths, and files carry it as the result's `text/plain` copy for Jupyter tools.
- **One numbering rule** in both apps: a session numbers its runs from 1; outputs loaded from a file keep their numbers but are marked stale, and the window shows them as such; `%oN` resolves only to a run in the current session.
- **One set of label rules.** In simple mode, `evaluate_expression` can run an expression as a cell of a notebook the client keeps itself: the client sends the cell's ID and the IDs of the code cells above it, and the server numbers the run and resolves `%oN` and a bare `%` by the same rules as the window's cells. The extension sends cell source unchanged and drops its own numbering and label rewriting.
- **Shared test notebooks** that the Rust serializer and the extension's serializer both run against, published with a TypeScript implementation of the spec for the extension (#17).
- **BREAKING (files):** files written by Aximar change shape: cell IDs, error outputs and kept metadata are added; `nbformat_minor` becomes 5 in the desktop app (it wrote 0). Older Aximar versions and Jupyter read them; older Aximar versions ignore the new parts.
- **BREAKING (MCP, VS Code):** the extension must send cell source unchanged with its cell IDs; its current rewriting stays broken until it does.

## Capabilities

### New Capabilities

- `notebook-format`: the `.macnb` file format and how Aximar reads and writes it: IDs, metadata, outputs, errors, MIME preference and stale outputs.

### Modified Capabilities

- `label-references`: numbering per session, stale loaded outputs, and cells of a notebook a client keeps itself.
- `notebook-files`: the window opens and saves through the backend, which records the path itself.

## Impact

- **Layers:** core (`notebooks/`, `notebook.rs`, `evaluation.rs`, `maxima/labels.rs`), MCP (`server.rs`, `batch.rs`, `convert.rs` removed, `params.rs`), Tauri (`commands/notebooks.rs`, `commands/notebook.rs`), frontend (`notebooks-client.ts`, `App.tsx`, `useNotebookEvents.ts`, `TemplateChooser.tsx`, cell output components), the VS Code extension (`serializer.ts`, `labels.ts`, `controller.ts`), and a new npm package.
- **Issues:** closes #6, #7, #8, #9, #10, #15, #16, #17 and #56; fixes the extension's label references, broken since #70.
- **MCP clients:** `evaluate_expression` gains an optional parameter; existing calls behave as now.
- **Saved notebooks:** see BREAKING above. Every notebook in `demos/notebooks` must survive an Aximar → VS Code → Aximar round trip unchanged (#16's exit check).

## Non-goals

- New output types (Plotly controls, animations) beyond those Aximar stores today.
- Package requirements (`metadata.aximar.requires`) themselves; this change only keeps the metadata, which S2 needs.
- Mirroring the extension's notebook on the server. The server records only the runs of the client's cells, by ID; the cells themselves stay in the client.
- Migrating old files on disk. Readers accept old shapes; files change when saved.

## Decisions

- **npm package name (#17):** `aximar-notebook-format`, unscoped, so no npm organisation is needed.
- **`text/plain` copies:** written, as Jupyter tools expect. A result's copy is Maxima's one-line form of it, which evaluation now captures; a plot's copy is a short description, never its source. Readers still ignore `text/plain` when a richer type is present.
