# Notebook file format

Status: **draft**, part of the OpenSpec change `s1-notebook-format`. The requirements are in `openspec/changes/s1-notebook-format/specs/notebook-format/spec.md`; this page describes the format field by field, for anyone writing a tool that reads or writes Maxima notebooks.

A Maxima notebook is a [Jupyter nbformat 4](https://nbformat.readthedocs.io/en/latest/format_description.html) JSON file. Aximar uses the extension `.macnb`; `.ipynb` files with the same content open the same way. The desktop app, its MCP server, batch runs (`aximar-mcp run`) and the VS Code extension all follow this page, and the test notebooks in `packages/aximar-notebook-format/fixtures/` check that they do. `fixtures/spec-example.macnb` shows every part of the format in one file.

## Rules for readers and writers

- **Write nbformat 4.5.** Read 4.0 to 4.5.
- **Keep what you don't understand.** Notebook metadata, cell metadata and raw cells a tool doesn't use go back into the file unchanged.
- **Write every cell's `id`**, and keep the IDs you read.
- **Never lose an output.** Each kind of output has one place in the file (see [Outputs](#outputs)), and reading it back gives the same output.

## Top level

```json
{
  "nbformat": 4,
  "nbformat_minor": 5,
  "metadata": {
    "kernelspec": { "name": "maxima", "display_name": "Maxima", "language": "maxima" },
    "aximar": { "title": "Orbits" }
  },
  "cells": []
}
```

| Field | Written as | Read |
|---|---|---|
| `nbformat` | `4` | must be 4 |
| `nbformat_minor` | `5` | 0 to 5 |
| `metadata.kernelspec` | the Maxima kernelspec above | any; replaced on write |
| `metadata.aximar` | see below | kept |
| other `metadata` keys | unchanged | kept |
| `cells` | see below | |

### `metadata.aximar`

| Key | Meaning |
|---|---|
| `template_id` | The built-in template the notebook came from, if any |
| `title` | A display title, if different from the file name |
| `description` | A short description, shown in the template chooser |
| `requires` | Reserved for S2 (packages a notebook needs). Kept today, not yet acted on |

Writers keep keys under `metadata.aximar` that they don't know, so a newer Aximar's keys survive an older one.

## Cells

```json
{
  "id": "k3Lr9wT2bQ",
  "cell_type": "code",
  "source": ["f(x) := x^2$\n", "f(3);"],
  "metadata": {},
  "execution_count": 2,
  "outputs": []
}
```

| Field | Written as | Read |
|---|---|---|
| `id` | always | used as the cell's ID when valid and unique, otherwise replaced |
| `cell_type` | `code`, `markdown` or `raw` | the same; a raw cell is kept as read-only text and written back unchanged |
| `source` | a list of lines, each ending in `\n` except the last | a string or a list of strings, joined |
| `metadata` | unchanged | kept |
| `execution_count` | the gutter number of the output, or `null` | shown on the stale output |
| `outputs` | code cells only; see below | |

### Cell IDs

nbformat 4.5 allows 1 to 64 characters of `a-z A-Z 0-9 - _`. A reader uses a cell's `id` when it fits that rule and no earlier cell in the file has it; otherwise it makes a new ID. A file with any made IDs counts as changed, so the next save writes good ones. Aximar's own IDs are 21-character nanoids, which fit the rule.

## Outputs

A code cell's output in Aximar can hold print text, a final value, plots, an image and an error. Each goes in its own nbformat output, in this order:

| Part | Output | Example |
|---|---|---|
| Print text, warnings | `stream`, `name: "stdout"` | `{"output_type": "stream", "name": "stdout", "text": ["Solving...\n"]}` |
| Final value | `execute_result` with `text/latex`, and `text/plain` (Maxima's one-line form) when known | `"data": {"text/latex": ["{{x^2}\\over{2}}"], "text/plain": ["x^2/2"]}` |
| Plotly plot | `display_data` with `application/x-maxima-plotly`, and `text/plain` `<Plotly figure>` | |
| SVG plot | `display_data` with `image/svg+xml`, and `text/plain` `<SVG plot>` | |
| PNG image | `display_data` with `image/png` (base64), and `text/plain` `<PNG image>` | |
| Error | `error`, `ename: "MaximaError"`, `evalue` the error text, `traceback: []` | |

Notes:

- The `execute_result`'s `execution_count` matches the cell's.
- `text/plain` copies are for tools that read nothing else, such as Jupyter's text views. A plot's copy is a description, never the plot's source.
- Text values are written as lists of lines, like `source`; readers accept a string too.
- The friendly explanation Aximar shows for an error is not stored: Aximar works it out again from `evalue` when it opens the file.

### Reading an output bundle

An `execute_result` or `display_data` holds one value in several MIME types. A reader takes one, in this order of preference:

1. `application/x-maxima-plotly`
2. `image/svg+xml`
3. `image/png`
4. `text/latex`
5. `text/plain`, only when none of the above is present

So a bundle with an SVG and a `text/plain` copy of the SVG source (as older versions of the VS Code extension wrote) reads as a plot, with no text.

`stream` outputs add to the cell's print text, in order. An `error` output marks the output as an error, with `evalue` as the message. Other output types are kept in the file but not shown.

## Stale outputs and numbering

Outputs read from a file came from a Maxima session that no longer exists:

- They are **stale** until their cell runs again. Aximar shows them dimmed, as from an earlier session.
- A session numbers its runs from 1. Opening a notebook starts its numbering again at 1, whatever numbers its outputs show, as Jupyter does.
- A label for a stale output has no value: `%o3` for a cell showing `Out[3]` from the file fails with "Out[3] (%o3) has no value in this session", and a bare `%` skips stale outputs. Run the cell first.

Maxima's own labels (`%o167`) are not stored. They mean nothing in a new session.

## A complete example

```json
{
  "nbformat": 4,
  "nbformat_minor": 5,
  "metadata": {
    "kernelspec": { "name": "maxima", "display_name": "Maxima", "language": "maxima" }
  },
  "cells": [
    {
      "id": "intro",
      "cell_type": "markdown",
      "source": ["# Integrals"],
      "metadata": {}
    },
    {
      "id": "c1",
      "cell_type": "code",
      "source": ["print(\"working\")$\n", "integrate(x, x);"],
      "metadata": {},
      "execution_count": 1,
      "outputs": [
        { "output_type": "stream", "name": "stdout", "text": ["working"] },
        {
          "output_type": "execute_result",
          "execution_count": 1,
          "data": { "text/latex": ["{{x^2}\\over{2}}"], "text/plain": ["x^2/2"] },
          "metadata": {}
        }
      ]
    },
    {
      "id": "c2",
      "cell_type": "code",
      "source": ["1/0;"],
      "metadata": {},
      "execution_count": 2,
      "outputs": [
        {
          "output_type": "error",
          "ename": "MaximaError",
          "evalue": "expt: undefined: 0 to a negative exponent.\n -- an error. To debug this try: debugmode(true);",
          "traceback": []
        }
      ]
    }
  ]
}
```

## Compatibility

- **Older Aximar versions** open these files: they ignore `id`, `error` outputs, `text/plain` copies and metadata they don't know, and lose those parts when they save.
- **Files from older Aximar versions** open normally: they get new cell IDs, and the next save writes the current format. The desktop app wrote `nbformat_minor` 0 and no IDs; aximar-mcp wrote 5 without IDs.
- **Files from older versions of the VS Code extension** open normally: their `text/plain` copies of plots are ignored, and their `error` outputs read as errors. The extension's cell metadata (`outputLabel`, `executionCount`) is kept but not used.
- **Jupyter** opens these files as notebooks for an unknown kernel, and shows each result's `text/plain` copy.
