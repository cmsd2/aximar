Each numbered group is one pull request. Aximar groups run `cargo test --workspace`, `cargo test -p aximar-mcp -- --ignored` (with Maxima) and `npm test`, and update the Unreleased section of `CHANGELOG.md` for changes MCP clients or saved files can see. Groups 6 and 7 run the extension's own tests.

## 1. Format spec and test notebooks (#15)

- [ ] 1.1 Write `docs/notebook-format.md`: nbformat 4.5 and cell IDs, kept metadata, the `metadata.aximar` keys, the output mapping and MIME preference, errors, stale outputs and numbering; verify it covers every requirement of the `notebook-format` spec
- [ ] 1.2 Add `packages/aximar-notebook-format/fixtures/` with notebooks for each requirement: written by Aximar, by the extension (with `text/plain` copies and `error` outputs), and by Jupyter (4.0 without IDs, with tags and other metadata); each with a short README line saying what it tests

## 2. One serializer in aximar-core (#16, #7, #9, #10, #6)

- [ ] 2.1 Add `metadata` to `Notebook` and `Cell`, kept through undo; let `NotebookMetadata` keep unknown keys; verify with unit tests that metadata survives undo and redo
- [ ] 2.2 Add `notebooks::format::{read, write}` with the ID rule, the output mapping, the MIME preference and `error` outputs; `CellOutput` gains `stale` and `error_info`; verify with unit tests per requirement, and by reading and writing every fixture
- [ ] 2.3 Recompute `error_info` for loaded error outputs with `errors::enhance_error_with_packages`; verify with a test that a loaded "0 to a negative exponent" error has the "Division by Zero" explanation

## 3. Aximar's callers use the serializer (#16)

- [ ] 3.1 aximar-mcp `open_notebook`, `save_notebook`, `load_template` and batch runs use `notebooks::format`; delete `convert.rs`; verify the existing MCP file tests pass, plus a test that a saved error output reopens with its error and the cell IDs are kept
- [ ] 3.2 Add Tauri `nb_open`, `nb_save` and `nb_load_template`; the window's open, save, save-as, command-line open, template chooser and welcome notebook use them; delete the TypeScript conversion in `notebooks-client.ts`, the raw `save_notebook` and `open_notebook` commands and `nb_load_cells`; verify with vitest tests of the new client calls, and by hand that opening, saving and templates work and the tab titles follow the file
- [ ] 3.3 Send `stale` and `error_info` to the window; show stale outputs as from an earlier session, and friendly explanations for errors (#56); verify with vitest tests that a stale output and an `error_info` reach the cell, and by hand that both show

## 4. Numbering and label contexts (#8)

- [ ] 4.1 Loading resets the notebook's numbering to 1; stale outputs never record labels; verify with core tests for both scenarios of the numbering requirement
- [ ] 4.2 `evaluate_expression` takes an optional `labels` context in MCP and the Tauri command; verify with aximar-mcp tests for both scenarios of the label-context requirement

## 5. npm workspace and TypeScript package (#17)

- [ ] 5.1 Add an npm workspace (`packages/*`) and `packages/aximar-notebook-format`, published unscoped as `aximar-notebook-format`, implementing the spec in TypeScript, tested against the fixtures with vitest
- [ ] 5.2 Run the Rust serializer's fixture tests in CI from the same folder; publish the package

## 6. The VS Code extension (vscode-extension repository)

- [ ] 6.1 Replace the extension's serializer with the package; write IDs, keep errors, stop adding `text/plain` copies; verify with the extension's tests against the fixtures
- [ ] 6.2 Send cell source unchanged with a `labels` context to `evaluate_expression`, delete `labels.ts`, and number runs from 1 per session with stale loaded outputs; verify with the extension's tests and by hand that `%o1` and `%` work in a VS Code notebook again

## 7. Round trip and close-out

- [ ] 7.1 Add the round-trip script and run it over every notebook in `demos/notebooks` (Aximar → extension → Aximar); verify it reports no differences
- [ ] 7.2 Update `docs/project-plan.md` (S1 done), `docs/mcp-server.md` (file behaviour and the `labels` parameter) and `docs/code-audit.md` (bug 8, the ipynb theme); archive this change with `openspec archive s1-notebook-format` and check that `openspec/specs/` gains `notebook-format`
