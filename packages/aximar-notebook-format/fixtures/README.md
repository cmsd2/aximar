# Notebook format test notebooks

Test notebooks for the notebook format (`docs/notebook-format.md`). The crate in the folder above reads and writes them in its tests, and its npm build (used by the VS Code extension) is checked against them too, so the desktop app, aximar-mcp and the extension handle the same files the same way.

| File | Written by | What it tests |
|---|---|---|
| `spec-example.macnb` | Hand-written in the current format | Every part of the format: cell IDs, kept notebook and cell metadata (including an unknown `metadata.aximar` key), a raw cell, print text, a result with `text/latex` and `text/plain`, SVG, Plotly and PNG plots with `text/plain` descriptions (the PNG data as a single string), and an `error` output. A read and write gives back the same file |
| `aximar-mcp-before-s1.macnb` | aximar-mcp `save_notebook`, before S1 (saved from the running app) | nbformat 4.5 without cell IDs; an empty `metadata.aximar`; the cell that failed was saved with no outputs (#7); a real gnuplot SVG |
| `aximar-window-before-s1.macnb` | The desktop app's save, before S1 | `nbformat_minor` 0, sources as single strings, no cell IDs, only the kernelspec in metadata (#9), and the failed cell saved with no outputs |
| `vscode-extension-before-s1.macnb` | The VS Code extension's serializer, before S1 | Every output written as an `execute_result` with a `text/plain` copy of its first MIME type: an SVG plot must read as a plot, not as its source (#6); a result's `text/plain` copy of its LaTeX must not read as its one-line form; print text as a `text/plain`-only bundle; an `error` output with `ename` `Error` (#7); the extension's own cell metadata (`outputLabel`, `executionCount`) kept but unused; no cell IDs |
| `jupyter-4.0.ipynb` | A Jupyter notebook, nbformat 4.0 | No cell IDs; a raw cell that must survive a round trip; `tags`, `collapsed`, `raw_mimetype` cell metadata and `language_info` and `jupytext` notebook metadata, all kept (#9); a result with only `text/plain` |
| `cell-ids.macnb` | Hand-written | The ID rule (#10): `a1` kept for the first cell and replaced for its duplicate; `b-2_c` kept; `bad id!`, an empty ID and a 65-character ID replaced; a cell with no ID given one |

Outputs read from any of these files are stale (see "Stale outputs and numbering" in the format document).
