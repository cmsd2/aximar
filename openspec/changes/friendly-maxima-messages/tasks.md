Each numbered group is one pull request, stacked on S1 group 3 (#82) until it merges. Each group runs `cargo test --workspace`, the Maxima-backed tests (`cargo test -p aximar-core -- --ignored`, `cargo test -p aximar-mcp -- --ignored`) and `npm test`, and updates the Unreleased section of `CHANGELOG.md` for changes MCP clients or users can see.

## 1. Message IDs and the English table

- [ ] 1.1 Add `maxima/messages.rs` with `Message`, the English table `maxima/messages/en.json`, and rendering of `{name}` placeholders; `ErrorInfo` gains `id` and `params`; verify with unit tests of rendering and of a missing parameter
- [ ] 1.2 Move every `errors.rs` pattern onto the table with its ID; `error.arg_count` names the function, its definition and the arguments found, and stops suggesting `describe` for user functions; verify that each existing `errors.rs` test passes with the same English text, plus the `error.arg_count` scenario
- [ ] 1.3 Write `docs/maxima-messages.md` with every ID, the Maxima text it covers, its parameters and English text; add the table-and-code agreement test

## 2. Questions become errors (no more hangs or leaks)

- [ ] 2.1 Override `retrieve` at session start, guarded by `fboundp`; delete `ASSUMPTION_QUESTION_RE`, `assumption_answer`, `drain_answering_questions`, the question branches in both readers and `AppError::AssumptionRequired`; verify with Maxima-backed tests (aximar-core, both paths) for the sign, yes/no, leak and `errcatch` scenarios
- [ ] 2.2 Add the `question.*` IDs, mapping each question template to its ID and parameters, with `question.other` for anything else; verify with unit tests per template and one in another wording
- [ ] 2.3 MCP reports a question as a cell error; verify with an aximar-mcp Maxima-backed test for the MCP scenario; check that the DAP server's tests still pass without its question handling

## 3. Every failure leaves an output, and Lisp errors keep their message

- [ ] 3.1 `evaluate_cell` sets an error output for timeouts and cancellations (`error.timeout`, `error.cancelled`); verify with Maxima-backed tests for both scenarios, and a vitest test that the window shows the cell's error output
- [ ] 3.2 Keep the whole Lisp condition on both paths, without SBCL's two lines; add `error.lisp_type`, `error.recursion`, `error.float_overflow` and `error.lisp`; verify with parser tests from captured SBCL output and Maxima-backed tests for both Lisp scenarios
- [ ] 3.3 Drop the debugger hint from error text on both paths; verify with parser tests and the divergent-integral scenario

## 4. Notes

- [ ] 4.1 Add `messages::notes` with the `note.*` IDs from the survey, `CellOutput.notes` filled after evaluation and on load, never written to files; verify with unit tests per note (including consecutive `rat` lines, print text that looks like a note, and `note.zero_power` only with a result) and the reopened-note scenario
- [ ] 4.2 Show notes in the window as note lines with a toggle for Maxima's lines, and text output before plots; verify with vitest tests of the note line and toggle, and by hand with the survey's cells
- [ ] 4.3 Add the survey's other errors (`error.log_zero`, `error.part_index`, `error.part_atom`, `error.assign_target`, `error.empty_list`, `error.not_polynomial`, `error.algsys_failed`) to the table and docs; verify with unit tests from the survey's text

## 5. Close-out

- [ ] 5.1 Check every scenario in the `maxima-messages` spec against the tests; run the survey cells in the window and over MCP; update `CHANGELOG.md`; archive the change
