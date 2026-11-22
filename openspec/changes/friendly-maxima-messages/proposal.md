## Why

Maxima talks to the user in four ways besides results: errors, notes printed next to a result, questions, and Lisp-level failures. Aximar explains some errors in plain language, but the rest come through raw, and three of them break:

- **Questions hang or leak.** Maxima asks "Is a positive, negative or zero?" or "Is a equal to -1?" and waits for an answer on stdin. Aximar recognises only sign questions. It answers them with `positive;` written behind the cell's own housekeeping lines, which Maxima reads first as invalid answers, so it asks again; the spare answers then run in the next cell and show up in its output as stray `positive` lines. Yes/no questions ("Is a equal to -1?", "Is n an integer?") aren't recognised at all: the cell waits for the 30-second timeout.
- **Questions and timeouts leave the cell empty.** The cell turns red, and the message, with its `assume()` hint, goes only to the log panel.
- **Lisp errors lose their cause.** The parser ends an error at the first blank line, and SBCL puts one right after "Maxima encountered a Lisp error:", so the actual message ("The value 1 is not of type LIST") is dropped and the explanation blames an internal fault.

Notes such as `rat: replaced 0.5 by 1/2 = 0.5`, `solve: using arc-trig functions to get a solution. Some solutions will be lost.` or `Principal Value` read like print output, use Maxima's internal names, and some mislead: `expt: undefined: 0 to a negative exponent.` appears as a passing note inside `quad_qags`, `romberg` and plotting while the computation carries on.

Translation is coming (#44). Maxima translates only some of its own text, and none of Aximar's. Friendly messages built as English strings in Rust would have to be rebuilt to translate them.

## What Changes

- **Questions become errors.** At session start Aximar replaces Maxima's `retrieve`, the one function every question goes through, with one that raises a Maxima error quoting the question. Every kind of question then stops the cell cleanly, with a friendly error that names the line that answers it, such as `assume(a > 0)` or `declare(n, integer)`. The answering and draining code in `process.rs` and `AppError::AssumptionRequired` go.
- **Timeouts and cancellations leave an error output in the cell**, with an explanation, instead of only a log entry.
- **Lisp errors keep their message**, without SBCL's "Automatically continuing" lines, and common ones (a value of the wrong type, runaway recursion, floating-point overflow) get explanations.
- **Notes become friendly note lines.** Recognised notes in a cell's text output show as one plain-language line in a note style, with Maxima's text a click away; others stay as Maxima wrote them. Text output shows before plots, in the order Maxima printed it.
- **Every friendly message has a stable ID and named parameters** (for example `question.sign` with `expr: a`), with its English text in one table in aximar-core. Errors and notes carry the ID, parameters and English text, so a later translation renders the ID in another language and nothing has to parse English.
- **A message table**, `docs/maxima-messages.md`, lists each ID, the Maxima text it covers, its parameters and its English text. Message IDs are public: translations and tests depend on them.
- **Error text drops Maxima's debugger hint**, ` -- an error. To debug this try: debugmode(true);`, which has no use in a notebook.
- **BREAKING (MCP):** `run_cell` and `run_all_cells` report a question as a cell error (`is_error: true`) instead of failing with "Maxima needs more information". `get_cell` gains `notes`, and `error_info` gains `id` and `params`.

## Capabilities

### New Capabilities

- `maxima-messages`: how Aximar shows Maxima's errors, notes and questions: message IDs and parameters, the English table, questions as errors, Lisp errors, notes, and timeouts.

### Modified Capabilities

None. `cell-execution` describes statuses and trust, which this change keeps.

## Impact

- **Layers:** core (`maxima/process.rs`, `maxima/errors.rs`, a new `maxima/messages.rs` and English table, `maxima/legacy/parser.rs`, `maxima/envelope/`, `evaluation.rs`, `notebook.rs`), MCP (`server.rs` result shapes), the DAP server (its question handling goes with `process.rs`'s), frontend (`CellOutput.tsx`, `EnhancedErrorOutput.tsx`, a note line component, types).
- **Depends on** S1 group 3 (#82), which sends `error_info` to the window and reworks `CellOutput.tsx`.
- **Issues:** touches #44 (localisation) and #56 (friendly errors reach the window, fixed in #82).
- **MCP clients:** see BREAKING above.
- **Saved notebooks:** unchanged in format. A question saves as an `error` output. Notes and explanations are not stored; Aximar works them out again from the saved text when it opens a file, as it does for `error_info`.

## Non-goals

- Answering questions interactively in the cell. A question is an error with the fix; the fix goes in the notebook, so a rerun doesn't ask again.
- Translating anything. This change makes messages translatable; #44's work picks the languages and the UI for them.
- `read()` and `readonly()`, which read stdin directly. They still can't be answered from a notebook.
- Explaining every message Maxima has. The table starts with the messages the survey in `design.md` found most often and grows over time.
- The kernel-events package. It passes questions and output through; nothing in it changes.
