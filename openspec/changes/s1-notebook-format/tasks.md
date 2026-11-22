Each numbered group is one pull request. Aximar groups run `cargo test --workspace`, `cargo test -p aximar-mcp -- --ignored` (with Maxima) and `npm test`, and update the Unreleased section of `CHANGELOG.md` for changes MCP clients or saved files can see. Groups 6 and 7 run the extension's own tests.

## 1. Format spec and test notebooks (#15)

- [x] 1.1 Write `docs/notebook-format.md`: nbformat 4.5 and cell IDs, kept metadata, the `metadata.aximar` keys, the output mapping and MIME preference, errors, stale outputs and numbering; verify it covers every requirement of the `notebook-format` spec
- [x] 1.2 Add `packages/aximar-notebook-format/fixtures/` with notebooks for each requirement: written by Aximar, by the extension (with `text/plain` copies and `error` outputs), and by Jupyter (4.0 without IDs, with tags and other metadata); each with a short README line saying what it tests

## 2. One serializer in aximar-core (#16, #7, #9, #10, #6)

- [x] 2.1 Add `metadata` to `Notebook` and `Cell`, kept through undo; add a raw cell type that the window shows read-only and never runs; let `NotebookMetadata` keep unknown keys; verify with unit tests that metadata and raw cells survive undo and redo, and that running a raw cell is refused
- [x] 2.2 Add `notebooks::format::{read, write}` with the ID rule, the output mapping, the MIME preference and `error` outputs; `CellOutput` gains `stale` and `error_info`; verify with unit tests per requirement, and by reading and writing every fixture
- [x] 2.3 Capture Maxima's one-line form of each final value as `CellOutput.text_plain`: on the legacy path next to the injected `tex(%)`, on the kernel-events path from the result envelope or by the same request; write it as the result's `text/plain` copy and a description for plots; verify with legacy parser tests for the new housekeeping output, and a Maxima-backed test that `x^2/2;` saves `text/plain` `x^2/2` on both paths
  - The kernel-events path needs kernel-events to build `text/plain` with `mstring`, as `string()` does; `mgrind` wraps long forms at `linel`
- [x] 2.4 Recompute `error_info` for loaded error outputs with `errors::enhance_error_with_packages`; verify with a test that a loaded "0 to a negative exponent" error has the "Division by Zero" explanation

## 3. Aximar's callers use the serializer (#16)

- [x] 3.1 aximar-mcp `open_notebook`, `save_notebook`, `load_template` and batch runs use `notebooks::format`; delete `convert.rs`; verify the existing MCP file tests pass, plus a test that a saved error output reopens with its error and the cell IDs are kept
- [x] 3.2 Add Tauri `nb_open`, `nb_save` and `nb_load_template`; the window's open, save, save-as, command-line open, template chooser and welcome notebook use them; delete the TypeScript conversion in `notebooks-client.ts`, the raw `save_notebook` and `open_notebook` commands and `nb_load_cells`; verify with vitest tests of the new client calls, and by hand that opening, saving and templates work and the tab titles follow the file
- [x] 3.3 Send `stale` and `error_info` to the window; show stale outputs as from an earlier session, and friendly explanations for errors (#56); verify with vitest tests that a stale output and an `error_info` reach the cell, and by hand that both show
- [x] 3.4 Show raw cells read-only in the window, with no run control; show a result's `text_plain` when it has no LaTeX (a Jupyter result with only `text/plain`); verify with vitest tests of both, and by hand with `fixtures/jupyter-4.0.ipynb`

## 4. Numbering and label contexts (#8)

- [x] 4.1 Loading resets the notebook's numbering to 1; stale outputs never record labels; verify with core tests for both scenarios of the numbering requirement
- [x] 4.2 Simple-mode `evaluate_expression` takes an optional `cell_id` and `cells_above`, runs the expression as that client cell with the notebook's label rules, and returns its `execution_count`; the `labels` parameter from #85 goes; verify with core tests of the client-cell rules and aximar-mcp tests (with Maxima) for each scenario of the client-cell requirement
- [x] 4.3 Simple-mode `evaluate_expression` without `cell_id` keeps Maxima's own labels, so clients that resolve labels themselves (the extension up to v0.3.0) keep working; its input schema shows `cell_id` and `cells_above` for clients to detect; verify with simple-mode tests (with Maxima) for the plain-expression and schema scenarios

## 5. The format crate and its npm package (#17)

- [x] 5.1 Move the format into the MIT crate `packages/aximar-notebook-format` (model, `read`, `write`, IDs), with aximar-core converting to and from `Notebook`; build it as WebAssembly with `wasm-pack` (feature `wasm`) as the npm package `aximar-notebook-format`, with generated TypeScript types; verify with the crate's tests against the fixtures, aximar-core's tests, and a Node smoke test that the npm build writes the same bytes
- [x] 5.2 Build the npm package and run its smoke test in CI; add a release workflow that publishes it to npm on a `notebook-format-v*` tag, through npm trusted publishing
- [x] 5.3 Publish the first version by hand, then make `release-notebook-format.yml` the package's trusted publisher on npmjs.com (by hand; see the package README)

## 6. The VS Code extension (vscode-extension repository)

- [ ] 6.1 Replace the extension's serializer with the package; write IDs, keep errors, and write `text/plain` copies by the spec's rule (the one-line result, or a plot description) instead of copying the first MIME type; verify with the extension's tests against the fixtures
- [ ] 6.2 Send cell source unchanged with the cell's ID and the IDs of the code cells above to `evaluate_expression`, show the returned `execution_count`, and delete `labels.ts` and the extension's own numbering and label map; verify with the extension's tests and by hand that `%o1` and `%` work in a VS Code notebook again
- [ ] 6.3 Check the server's `tools/list` for `cell_id` on `evaluate_expression`; when it's missing, say aximar-mcp needs updating, offer the download, and don't run cells; make the download pick a release with the features the extension needs; verify with the extension's tests of the check

## 7. Round trip and close-out

- [ ] 7.1 Add the round-trip script and run it over every notebook in `demos/notebooks` (Aximar → extension → Aximar); verify it reports no differences
- [ ] 7.2 Update `docs/project-plan.md` (S1 done), `docs/mcp-server.md` (file behaviour, and `cell_id` and `cells_above`) and `docs/code-audit.md` (bug 8, the ipynb theme); archive this change with `openspec archive s1-notebook-format` and check that `openspec/specs/` gains `notebook-format`
