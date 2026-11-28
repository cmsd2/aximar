## 1. Run states, printed text and silent cells (one PR)

- [x] 1.1 Add the `cellStyle` toolbar global to `.storybook/preview.tsx` (Card, Bracket; a story's own parameter wins); verify by opening a cell story in each style in Storybook
- [x] 1.2 Add `ends_silent(input)` to `crates/aximar-core/src/maxima/protocol.rs`, using `strip_trailing_comments` and `find_terminators`; verify with Rust tests for `x$`, `x;`, `f(x) := x$ /* helper */`, a `$` inside a string, and a `$` inside a comment
- [x] 1.3 Add `silent: bool` to `SyncCell` in `src-tauri/src/mcp/sync.rs`, filled from each cell's input; verify with a Rust test that a cell ending in `$` reports silent and one ending in `;` doesn't
- [x] 1.4 In `CellOutput.tsx`, show nothing for a silent cell with nothing to show, and keep "(no output)" otherwise; verify with vitest tests of both, and a story in both styles
- [x] 1.5 Show In [\*] for a running cell and In [·] for a queued one in `Cell.tsx`, with the accent bracket or border for running in `global.css`; verify with a vitest test of the labels and the Outputs "Running and queued" story in both styles
- [x] 1.5a Add `CellStatus::Queued` to `aximar-core`: `run_all` marks the cells with code it will run queued, reporting it once, and gives each cell still queued at the end its old status; verify with `tests/run_all_queue.rs` (a run stopped by the trust check, no Maxima) and the ignored `cells_queue_then_run_in_turn` in `tests/run_all.rs`
- [x] 1.6 Move printed text (`.text-output`, `RichTextOutput`'s text) to `--text-primary`; verify with the contrast scan over the cell stories in both themes

## 2. Errors (one PR)

- [x] 2.1 Add `IconAlertCircle` and `IconBulb` to `ui/icons.ts`; verify with the icons-list test and the Foundations story
- [x] 2.2 Restyle `EnhancedErrorOutput`: red icon before a title in the normal text colour, explanation in secondary text, lightbulb before the suggestion with no box or bar, "Show Maxima's error" and "Hide Maxima's error"; verify with vitest tests that the title has no error colour class and the suggestion has no box, and the Errors stories in both styles and themes
- [x] 2.3 Give the unexplained error the same header (icon, then Maxima's text in a neutral `<pre>`); verify with a vitest test and the "Maxima error without explanation" story
- [x] 2.4 Check that red shows only on the bracket or border: run the contrast scan and look at the Cells "Notebook" and Errors stories in bracket and card style, light and dark

## 3. Code in explanations and notes (one PR)

- [x] 3.1 In `lib/messages.ts`, render a message as prose and code spans: call Lingui with sentinels for the parameters, split the translation on backticks, then put the values back; keep a plain-string variant; verify with vitest tests for a code span, a parameter inside one, a value containing a backquote, an unbalanced backtick, and a message with no backticks
- [x] 3.2 Add backticks around code in the English templates of the explanations and notes in `crates/aximar-core` (function names, expressions, values), regenerate `docs/maxima-messages.md` and the catalogs; verify with `cargo test -p aximar-core` (the message tests and `tests/maxima_messages.rs`), `scripts/i18n-extract --check`, and a list of the changed IDs in the PR
- [x] 3.3 Make `scripts/i18n-extract --check` warn when a translation's backtick count differs from its English, and add a line to the translator guide in `docs/i18n.md`; verify by adding a translation with a missing backtick to a test catalog and seeing the warning
- [x] 3.4 Restyle notes in `TextWithNotes.tsx`: Tabler info icon, code spans from 3.1, and a quiet "Show Maxima's text" button without an underline; verify with the vitest notes tests and the Notes "All notes" story in both styles
- [x] 3.5 Check that MCP results carry the backticks: a test in `crates/aximar-mcp` that a cell giving `note.rat_replaced` returns text with "`0.5 → 1/2`" (needs a live Maxima, so `#[ignore]` with the others)

## 4. Suggestions (one PR)

- [x] 4.1 Remove the band under results; move the suggestion logic into a `useCellSuggestions` hook that keeps only suggestions that insert a cell
- [x] 4.2 Draw every gap in both styles as a line with one + near its left end; its menu lists Code, Markdown, then "From Out [n]" and the result above's suggestions; Tab at the caret opens it; only one gap shows its line and + at a time; the menu is opaque over the cells; the caret's left end lines up with the input and output; verify with vitest tests (`suggestions.test.tsx`, `cell-gap.test.tsx`) and the Outputs "Next Steps Menu" story in both styles
- [x] 4.3 Move plot saving to the output's hover toolbar (SVG for a gnuplot plot; SVG, PNG and JSON for a Plotly chart, at the bottom right) through `lib/plot-save.ts`, shared with the plot viewer; verify with `plot-actions.test.tsx`
- [x] 4.4 Keep the Settings switch working, and list Tab at the caret in Keyboard Shortcuts; verify with the "Suggestions Off" story and its vitest test

## 5. Docs and the gallery

- [x] 5.1 Update `docs/ui-audit.md` (items 8 to 11 and 22, under "Changed since") and `docs/design-library.md` if the quiet text button or the error header become shared parts; verify the links and anchors resolve
- [x] 5.2 Regenerate the gallery and catalogs (`npm run i18n:gallery`, `scripts/i18n-extract`), and check the cell stories in both styles and themes; verify `scripts/i18n-extract --check` passes
- [x] 5.3 By hand in the app with Maxima: run `1/0;`, `frobnicate(1);`, `rat(0.5*x);`, `print("n =", 3)$`, a definition ending in `$`, and Run All over a slow cell, in bracket and card style (checked by hand on macOS, 28 November 2026)
