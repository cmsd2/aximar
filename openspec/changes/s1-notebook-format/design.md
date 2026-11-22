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
| `latex` | `execute_result` with `text/latex` and the execution count |
| `plot_data` | `display_data` with `application/x-maxima-plotly` |
| `plot_svg` | `display_data` with `image/svg+xml` |
| `image_png` | `display_data` with `image/png` |
| `is_error` | `error`, `ename` `MaximaError`, `evalue` the error text |

Reading takes one value from each `execute_result` or `display_data` bundle, preferring Plotly, then SVG, then PNG, then LaTeX, and `text/plain` only when the bundle has nothing richer (#6). `stream` text appends to `text_output`; an `error` output sets `is_error` and `error`.

Friendly explanations (`error_info`, #56) are not stored in the file. They come from the error text and the catalog, so the backend computes them again when it loads an error output, and `CellOutput` carries them to the window.

### 5. Numbering and stale outputs

`CellOutput` gains `stale: bool`. Outputs read from a file are stale; running the cell replaces its output with a fresh one. The window shows stale outputs as from an earlier session (dimmed, with the gutter number in a muted style).

A session numbers its runs from 1, as Jupyter and the extension do. Loading a notebook resets the notebook's numbering, so its first run is `In[1]` whatever the loaded outputs show. A stale output never records a label, so `%oN` for it fails as #59 made it; the label-references spec gets a scenario for it.

### 6. One label resolver: a label context on expressions

`evaluate_expression` takes an optional `labels` object: `label_map` (gutter number → Maxima label) and `previous_output_label`. When it is given, the expression is prepared as a cell with that context, a bare `%` included; when it isn't, the expression uses the notebook's own labels, as now.

The extension then sends a cell's source unchanged with its label map, and deletes its port of the rewriter (`labels.ts`). This restores its label references, broken since #70, and leaves one rewriter, which already handles strings, comments and labels with no value.

*Alternative:* run the extension's cells through the server's notebook model (`add_cell`, `run_cell`). It removes the extension's own state too, but it is a much larger change to the extension.

### 7. The window opens and saves through the backend

New Tauri commands:

- `nb_open(path)`: creates a notebook, reads the file with the serializer, loads it, records the path (group 7's `PathChanged`) and returns the notebook ID.
- `nb_save(notebook_id, path)`: writes the notebook with the serializer and records the path. The window still shows the file dialogs and passes the chosen path.
- `nb_load_template(notebook_id, template_id)`: loads a built-in template through the same reader.

The TypeScript conversion in `notebooks-client.ts`, the raw `save_notebook` and `open_notebook` commands, and `nb_load_cells` go. Opening Plotly JSON files stays in the frontend.

### 8. Shared test notebooks and the TypeScript package

An npm workspace (`packages/*`) holds `packages/notebook-format`: a TypeScript implementation of the spec for the extension, and `fixtures/`, test notebooks covering each requirement (written by Aximar, by the extension, and by Jupyter). The Rust serializer's tests read the same fixtures from that folder. The extension depends on the published package and replaces its serializer with it.

### 9. The round-trip check

A script runs every notebook in `demos/notebooks` through Aximar's serializer, then the extension's, then Aximar's again, and compares the results with the originals. It is #16's exit check, and it runs before this change is archived.

## What stays in core and what the adapters keep

Core: the serializer, the output mapping, IDs, metadata, stale outputs, numbering and label resolution. Tauri and MCP only take paths and IDs and report results; the window keeps its file dialogs and display. The extension keeps its own serializer, implemented from the same spec and checked against the same fixtures.

## What changes for MCP clients and saved notebooks

- `evaluate_expression`: a new optional `labels` parameter.
- `open_notebook`, `save_notebook`, `load_template`: same parameters and results; files gain IDs, errors and kept metadata.
- `list_cells` and `get_cell`: cell IDs are the file's IDs after opening a file.
- Files: see the proposal's BREAKING notes.

## Risks / Trade-offs

- [Older Aximar versions reading new files] → They ignore `id`, `error` outputs and unknown metadata, so they open the file and lose those parts on save, as today.
- [The extension stays broken until it adopts the label context] → The extension's group lands with this change's core groups; the changelog and the extension's release notes say so.
- [Files with duplicate or invalid IDs] → The reader makes new IDs and the notebook counts as changed, so the next save fixes the file.
- [`stale` must not outlive a run] → `SetCellOutput` always writes a fresh output; a test covers it.

## Migration Plan

Each task group is one pull request. The spec and fixtures land first; then the core serializer; then the three Aximar callers; then the numbering and label changes; then the extension and the npm package. Rolling back a group means reverting its PR; files written in between stay readable by every version.
