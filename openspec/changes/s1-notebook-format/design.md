## Context

Three serializers read and write `.macnb` (nbformat 4 JSON): `src/lib/notebooks-client.ts` in the window, `crates/aximar-mcp/src/convert.rs` for MCP and batch runs, and `src/notebook/serializer.ts` in the VS Code extension. aximar-core has the file types (`notebooks/types.rs`) and plain read and write (`notebooks/io.rs`), but no conversion to or from its `Notebook` model. The window converts in TypeScript and sends cells to `nb_load_cells`; MCP converts in Rust.

The core model (`notebook.rs`) has no place for cell metadata, notebook metadata or friendly error explanations, and its cell IDs are nanoids made on every load.

## Goals / Non-Goals

**Goals:** one format, written down; one Rust serializer for everything Aximar runs; the same rules in the extension, checked against the same test notebooks; no loss in a round trip between the apps.

**Non-Goals:** see the proposal.

## Decisions

### 1. The format is a document plus a spec

`docs/notebook-format.md` is the reference for people writing tools: field by field, with examples. The `notebook-format` capability states the requirements as testable scenarios. The same split worked for Unicode input (`docs/unicode-input.md` and its spec).

### 2. One serializer in aximar-core

`notebooks::format` offers:

- `read(json: &str) -> Result<NotebookFile, FormatError>`: the cells (ID, type, source, metadata, output), the notebook metadata, and whether IDs had to be made.
- `write(notebook: &Notebook) -> String`.

`Notebook` gains `metadata` (a JSON object) and `Cell` gains `metadata`; both are kept through undo, and anything Aximar doesn't own passes through untouched. `NotebookMetadata` in `types.rs` keeps unknown keys with `#[serde(flatten)]`.

`LoadCells` takes `NotebookFile`'s cells instead of tuples, so all three callers (window, MCP, batch) load the same way. `convert.rs` and `ipynb_to_cell_tuples` go.

*Alternative:* keep the TypeScript serializer and make it match. Two implementations of one spec drift, as these did; the window can read and write through the backend as MCP does.

### 3. Cell IDs

Readers use a cell's `id` as its internal ID when it is valid (1 to 64 characters of `a-z A-Z 0-9 - _`, as nbformat 4.5 requires) and unique in the file; otherwise they make one. Writers always write `id`. Aximar's nanoids already fit the rule, so new cells need no change.

### 4. Outputs

One cell output in the core model maps to several nbformat outputs, in this order:

| Core field | nbformat output |
|---|---|
| `text_output` | `stream`, name `stdout` |
| `latex`, `text_plain` | `execute_result` with `text/latex`, a `text/plain` copy when `text_plain` is known, and the execution count |
| `plot_data` | `display_data` with `application/x-maxima-plotly`, and `text/plain` `<Plotly figure>` |
| `plot_svg` | `display_data` with `image/svg+xml`, and `text/plain` `<SVG plot>` |
| `image_png` | `display_data` with `image/png`, and `text/plain` `<PNG image>` |
| `is_error` | `error`, `ename` `MaximaError`, `evalue` the error text |

Reading takes one value from each `execute_result` or `display_data` bundle, preferring Plotly, then SVG, then PNG, then LaTeX, and `text/plain` only when the bundle has nothing richer (#6). `stream` text appends to `text_output`; an `error` output sets `is_error` and `error`.

`CellOutput` gains `text_plain`: Maxima's one-line form of the final value, as `string(%)` gives it. The legacy path captures it next to the `tex(%)` it already injects; the kernel-events path takes it from the result envelope, or asks for it the same way when the envelope lacks it. It is shown nowhere in the window; it exists for files, so Jupyter and other tools that only read `text/plain` see each result. Outputs without it (older files, results with no value) write no `text/plain` copy.

Friendly explanations (`error_info`, #56) are not stored in the file. They come from the error text and the catalog, so the backend computes them again when it loads an error output, and `CellOutput` carries them to the window.

### 5. Numbering and stale outputs

`CellOutput` gains `stale: bool`. Outputs read from a file are stale; running the cell replaces its output with a fresh one. The window shows stale outputs as from an earlier session (dimmed, with the gutter number in a muted style).

A session numbers its runs from 1, as Jupyter and the extension do. Loading a notebook resets the notebook's numbering, so its first run is `In[1]` whatever the loaded outputs show. A stale output never records a label, so `%oN` for it fails as #59 made it; the label-references spec gets a scenario for it.

### 6. One set of label rules: the server records a client's cell runs

In simple mode, `evaluate_expression` takes an optional `cell_id` and `cells_above` (the IDs of the code cells above it, in order). With them the expression runs as that cell: the server gives the run the next gutter number, prepares the input as a cell with the session's labels, takes a bare `%` from the nearest cell in `cells_above` whose latest run in this session produced an output, records the run's label against the cell, and returns the number as `execution_count`. Without them the expression runs as now.

These are the window's rules, in the same `Notebook` methods: a cell that failed, printed nothing, or hasn't run in this session (one reopened from a file) is never "the output above", a restart or a new MCP process starts numbering at 1, and `%oN` for a number not run in this session fails. The cells stay in the client; the server keeps only, per cell ID, its latest run.

The extension then sends a cell's source unchanged with its ID (VS Code's cell URI, fixed while the notebook is open) and the IDs above it, shows the returned number, and deletes its numbering, its label map and its port of the rewriter (`labels.ts`). This restores its label references, broken since #70.

*Alternative, shipped briefly in #85:* the client sends its own label map and previous output label. That shared the rewriter but left numbering and the choice of "the output above" in the extension, whose rules already differed from the window's (it reused labels saved in the file, stopped at a failed cell, and kept counting across MCP restarts). It also put client-supplied labels into the code Maxima runs, after the trust check.

*Alternative:* run the extension's cells through the server's notebook model (`add_cell`, `run_cell`). It removes the extension's own state too, but it is a much larger change to the extension.

### 7. The window opens and saves through the backend

New Tauri commands:

- `nb_open(path)`: creates a notebook, reads the file with the serializer, loads it, records the path (group 7's `PathChanged`) and returns the notebook ID.
- `nb_save(notebook_id, path)`: writes the notebook with the serializer and records the path. The window still shows the file dialogs and passes the chosen path.
- `nb_load_template(notebook_id, template_id)`: loads a built-in template through the same reader.

The TypeScript conversion in `notebooks-client.ts`, the raw `save_notebook` and `open_notebook` commands, and `nb_load_cells` go. Opening Plotly JSON files stays in the frontend.

### 8. Shared test notebooks and the TypeScript package

An npm workspace (`packages/*`) holds `packages/aximar-notebook-format`, published as the unscoped npm package `aximar-notebook-format`: a TypeScript implementation of the spec for the extension, and `fixtures/`, test notebooks covering each requirement (written by Aximar, by the extension, and by Jupyter). The Rust serializer's tests read the same fixtures from that folder. The extension depends on the published package and replaces its serializer with it.

### 9. The round-trip check

A script runs every notebook in `demos/notebooks` through Aximar's serializer, then the extension's, then Aximar's again, and compares the results with the originals. It is #16's exit check, and it runs before this change is archived.

## What stays in core and what the adapters keep

Core: the serializer, the output mapping, IDs, metadata, stale outputs, numbering and label resolution. Tauri and MCP only take paths and IDs and report results; the window keeps its file dialogs and display. The extension keeps its own serializer, implemented from the same spec and checked against the same fixtures.

## What changes for MCP clients and saved notebooks

- `evaluate_expression` in simple mode: new optional `cell_id` and `cells_above` parameters, and an `execution_count` in the result.
- `open_notebook`, `save_notebook`, `load_template`: same parameters and results; files gain IDs, errors and kept metadata.
- `list_cells` and `get_cell`: cell IDs are the file's IDs after opening a file.
- Files: see the proposal's BREAKING notes.

## Risks / Trade-offs

- [Older Aximar versions reading new files] → They ignore `id`, `error` outputs and unknown metadata, so they open the file and lose those parts on save, as today.
- [The extension stays broken until it sends its cell IDs] → The extension's group lands with this change's core groups; the changelog and the extension's release notes say so.
- [Files with duplicate or invalid IDs] → The reader makes new IDs and the notebook counts as changed, so the next save fixes the file.
- [`stale` must not outlive a run] → `SetCellOutput` always writes a fresh output; a test covers it.
- [Capturing `string(%)` adds a housekeeping statement to each run] → It runs only after the user's input succeeds, like `tex(%)`, its output is filtered like `tex(%)`'s, and the legacy parser's tests gain cases for it. A huge result doubles in size in the file, which is the cost of Jupyter compatibility.

## Migration Plan

Each task group is one pull request. The spec and fixtures land first; then the core serializer; then the three Aximar callers; then the numbering and label changes; then the extension and the npm package. Rolling back a group means reverting its PR; files written in between stay readable by every version.
