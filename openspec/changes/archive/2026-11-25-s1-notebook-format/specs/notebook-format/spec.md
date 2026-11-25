# notebook-format Specification

## Purpose

The `.macnb` notebook file format, and how Aximar reads and writes it from the desktop window, MCP and batch runs. `docs/notebook-format.md` describes the format field by field; these requirements state what readers and writers must do. The VS Code extension implements the same requirements.

## ADDED Requirements

### Requirement: Files are nbformat 4.5

A notebook SHALL be written as nbformat 4.5 JSON (`nbformat` 4, `nbformat_minor` 5) with the Maxima kernelspec, whichever front end writes it. Readers SHALL accept nbformat 4.0 to 4.5.

#### Scenario: Saved from the window

- **WHEN** the window saves a notebook
- **THEN** the file SHALL have `nbformat` 4 and `nbformat_minor` 5

#### Scenario: Older file

- **WHEN** a reader opens an nbformat 4.0 file with no cell IDs
- **THEN** it SHALL load every cell

### Requirement: Cells keep stable IDs

Every cell SHALL be written with an `id`. A reader SHALL use a cell's `id` as its ID when the ID is 1 to 64 characters of letters, digits, `-` and `_` and no other cell in the file has it; otherwise it SHALL make a new ID.

#### Scenario: Round trip keeps IDs

- **WHEN** a notebook whose cells have IDs `a1` and `b2` is opened and saved
- **THEN** the saved cells SHALL have IDs `a1` and `b2`, and `list_cells` over MCP SHALL report them

#### Scenario: Duplicate ID

- **WHEN** a file has two cells with the ID `x`
- **THEN** the reader SHALL keep `x` for the first and make a new ID for the second

### Requirement: Metadata is kept

Notebook metadata and cell metadata that Aximar doesn't use SHALL be written back unchanged.

#### Scenario: Metadata from another tool

- **WHEN** a file has notebook metadata `{"jupytext": {"formats": "ipynb"}}` and a cell with metadata `{"tags": ["setup"]}`, and it is opened and saved without changes
- **THEN** the saved file SHALL have the same notebook and cell metadata

### Requirement: Raw cells are kept

A `raw` cell SHALL be kept, shown as read-only text, and written back with its source and metadata unchanged.

#### Scenario: Raw cell from Jupyter

- **WHEN** a Jupyter notebook with a raw cell is opened and saved
- **THEN** the saved file SHALL have the raw cell, in the same place, with the same source and metadata

### Requirement: Outputs map to nbformat output types

A cell's output SHALL be written as: print output as a `stream` named `stdout`; the final value as an `execute_result` with `text/latex`, a `text/plain` copy holding Maxima's one-line form of the value when it is known, and the execution count; each plot as a `display_data` with `application/x-maxima-plotly`, `image/svg+xml` or `image/png`, and a `text/plain` copy that describes the plot without its source; and an error as an `error` output with `ename` `MaximaError` and the error text as `evalue`. Reading SHALL give back the same output.

#### Scenario: Round trip of every kind of output

- **WHEN** a cell's output has print text, a LaTeX result and an SVG plot, and the notebook is saved and opened again
- **THEN** the cell SHALL have the same print text, LaTeX and SVG

#### Scenario: Error output

- **WHEN** a cell that failed with `expt: undefined: 0 to a negative exponent.` is saved and opened again
- **THEN** the cell SHALL show that error, and the window SHALL show the friendly explanation for it

#### Scenario: Plain-text copy for Jupyter tools

- **WHEN** a cell runs `x^2/2;` and the notebook is saved
- **THEN** its `execute_result` SHALL hold `text/latex` and `text/plain` `x^2/2`, and a tool that reads only `text/plain` SHALL show `x^2/2` (needs Maxima)

#### Scenario: Plot has no source in its plain-text copy

- **WHEN** a cell with an SVG plot is saved
- **THEN** the plot's `display_data` SHALL have a `text/plain` copy that doesn't contain the SVG source

### Requirement: Readers prefer the richest type in a bundle

Within one `execute_result` or `display_data` bundle, a reader SHALL take one value, preferring `application/x-maxima-plotly`, then `image/svg+xml`, then `image/png`, then `text/latex`, then `text/plain`. `text/plain` SHALL be used only when the bundle has none of the others.

#### Scenario: Plot saved by the VS Code extension

- **WHEN** a bundle holds `image/svg+xml` and a `text/plain` copy of the same SVG source
- **THEN** the cell SHALL show the plot, and no text

#### Scenario: Result saved by the VS Code extension

- **WHEN** an `execute_result` holds `text/latex` and a `text/plain` with the same text
- **THEN** the reader SHALL take the LaTeX and no one-line form, so the copy isn't written back as one

### Requirement: Loaded outputs are stale

Outputs read from a file SHALL be marked stale until their cell runs again. The window SHALL show stale outputs as coming from an earlier session.

#### Scenario: Opening a notebook with outputs

- **WHEN** a notebook whose cells have outputs is opened
- **THEN** every output SHALL be stale, and running a cell SHALL replace its output with one that isn't

### Requirement: Both apps read each other's files without loss

A notebook written by Aximar and saved again by the VS Code extension, or the other way round, SHALL keep its cells, IDs, metadata and outputs.

#### Scenario: Demo notebooks

- **WHEN** every notebook in `demos/notebooks` goes through Aximar's serializer, the extension's, and Aximar's again
- **THEN** each result SHALL equal what Aximar's serializer wrote the first time
