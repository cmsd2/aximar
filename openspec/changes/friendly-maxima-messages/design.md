## Context

**Errors.** `maxima/errors.rs` matches a raw error against a dozen patterns and builds an `ErrorInfo` (title, explanation, suggestion, example) in English. The legacy parser collects error lines into `EvalResult.error`; the kernel-events overlay takes them from error envelopes. The window shows `error_info` in `EnhancedErrorOutput`, with Maxima's text behind "Show raw error".

**Questions.** Maxima asks through one Lisp function, `retrieve` (`src/macsys.lisp`), which prints the question to `*query-io*` and reads an answer with `mread`. Its callers: `ensure-sign` (sign questions, five forms), `$askequal` ("Is a equal to b?"), `ask-prop` ("Is n an integer?"), and a rare one in `hayat.lisp`. Each loops until it gets a valid answer.

Aximar writes each cell as one block: the expression, then housekeeping lines (the one-line form and `tex(%)`, the label print, the end sentinel). `process.rs` matches sign questions with `ASSUMPTION_QUESTION_RE`, writes `positive;` and the sentinel again, drains until the sentinel, and returns `AppError::AssumptionRequired`. Maxima reads the queued housekeeping lines first, as invalid answers, and asks again; each repeat gets another `positive;`, and the spares run as expressions in the next cell. Yes/no questions don't match the pattern, so Maxima re-asks until the timeout interrupts it. `read_dap_response` repeats the same logic for the debugger.

**Notes.** Anything Maxima prints that isn't an error, a label or LaTeX goes into `text_output`, which the window shows as monospace text after any plot and before the result.

**Lisp errors.** SBCL prints "Maxima encountered a Lisp error:", a blank line, the condition over several lines, a blank line, and "Automatically continuing. / To enable the Lisp debugger set *debugger-hook* to nil." The parser ends an error at the first blank line.

### Survey

Running likely triggers in Maxima 5.50 (SBCL) gave these messages, which set the first table:

| Kind | Maxima prints | Proposed ID |
|---|---|---|
| Question | Is a positive, negative or zero? | `question.sign` |
| Question | Is a positive or zero? / negative or zero? / positive or negative? | `question.sign_pz`, `question.sign_nz`, `question.sign_pn` |
| Question | Is a zero or nonzero? | `question.zero` |
| Question | Is a equal to -1? | `question.equal` |
| Question | Is n an integer? (any `ask-prop` property) | `question.property` |
| Note | rat: replaced 0.5 by 1/2 = 0.5 | `note.rat_replaced` |
| Note | solve: using arc-trig functions to get a solution. Some solutions will be lost. | `note.solve_arc_trig` |
| Note | solve: dependent equations eliminated: (2) | `note.solve_dependent` |
| Note | Principal Value | `note.principal_value` |
| Note | plot2d: expression evaluates to non-numeric value somewhere in plotting range. | `note.plot_non_numeric` |
| Note | quad_qags: Numerical evaluation of f at 0.0 is not a float or is not defined | `note.quadrature_undefined` |
| Note | expt: undefined: 0 to a negative exponent. (in a cell that still has a result) | `note.zero_power` |
| Note | tellsimp: warning: rule will treat '+' as noncommutative and nonassociative. | `note.tellsimp_noncommutative` |
| Error | The value 1 is not of type LIST (Lisp) | `error.lisp_type` |
| Error | Control stack exhausted (Lisp) | `error.recursion` |
| Error | arithmetic error FLOATING-POINT-OVERFLOW signalled | `error.float_overflow` |
| Error | log: encountered log(0). | `error.log_zero` |
| Error | inpart: fell off the end. / part: fell off the end. | `error.part_index` |
| Error | part: argument must be a non-atomic expression; found x | `error.part_atom` |
| Error | assignment: cannot assign to 2 | `error.assign_target` |
| Error | first: empty argument. | `error.empty_list` |
| Error | allroots: expected a polynomial; found sin(x) | `error.not_polynomial` |
| Error | algsys: Couldn't reduce system to a polynomial in one variable. | `error.algsys_failed` |

The existing `errors.rs` patterns keep their meaning and get IDs: `error.division_by_zero`, `error.syntax`, `error.incomplete_input`, `error.arg_count`, `error.undefined_variable`, `error.unknown_function`, `error.divergent_integral`, `error.inconsistent_equations`, `error.matrix_dimension`, `error.package_not_found`, `error.lisp`. Timeouts and cancellations get `error.timeout` and `error.cancelled`; a question Aximar can't classify gets `question.other`.

## Goals / Non-Goals

**Goals:** no question ever hangs a cell or leaks into another; every failure leaves an output in its cell; Maxima's notes read as plain language; every friendly message is addressable by a stable ID, ready for translation.

**Non-Goals:** see the proposal.

## Decisions

### 1. Questions raise an error inside Maxima

Session start defines, through the existing `:lisp` init line:

```lisp
(defun maxima::retrieve (msg flag)
  (declare (ignore flag))
  (maxima::merror "Maxima needs to know: ~M" msg))
```

`merror` unwinds to the top level like any Maxima error, so the cell's housekeeping lines run as usual, the end sentinel arrives, and the cell reports an error. Nothing is written to stdin after the cell, so nothing leaks. `errcatch` catches it like any other error. The override applies to every session `MaximaProcess` starts: the window, MCP, batch runs and the DAP server.

`ASSUMPTION_QUESTION_RE`, `assumption_answer`, `drain_answering_questions`, the question branches of both readers, and `AppError::AssumptionRequired` go.

*Alternatives:* answer questions automatically (what Aximar does now, and the cause of the leak); answer interactively in the cell (a non-goal: MCP and batch runs can't, and the answer isn't in the notebook); send housekeeping lines only after the expression finishes (fixes the leak, not the yes/no hang, and doubles the round trips).

*Risk:* `retrieve` is internal to Maxima. It has had this name and signature since the Macsyma sources; the init line checks `fboundp` first, and a Maxima-backed test asks each kind of question.

### 2. Messages have IDs, parameters and an English table

```rust
pub struct Message {
    pub id: String,                       // "question.sign"
    pub params: BTreeMap<String, String>, // {"expr": "a"}
    pub text: String,                     // the English line, rendered from the table
}
```

`ErrorInfo` gains `id` and `params`; its `title`, `explanation`, `suggestion` and `example` stay, rendered in English from the table, so MCP clients and the window read text as they do now. The table is `crates/aximar-core/src/maxima/messages/en.json`: one entry per ID with `title`, `text`, `suggestion` and `example`, each a template with `{name}` placeholders. A unit test checks that every ID the code produces has an entry, that every placeholder in an entry has a parameter, and that `docs/maxima-messages.md` lists every ID.

Translation later adds a table per language and renders `id` and `params` with it, in whichever layer #44 chooses. Nothing ever matches English text Aximar produced.

*Alternative:* English strings in Rust, as now. Each would have to be found, keyed and parameterised during translation, and a renamed sentence would break nothing visible.

### 3. Recognition happens in aximar-core, on both paths and on load

`messages::explain_error(raw, catalog, packages) -> Option<ErrorInfo>` replaces `enhance_error_with_packages`, and `messages::notes(text_output) -> Vec<Note>` finds notes. A `Note` is a `Message` plus the range of `text_output` lines it covers. `CellOutput` gains `notes`, filled from an evaluation and by `notebooks::format::explain_errors` (renamed `explain`) when a file is opened. Notes are never written to files.

Each question template maps to an ID by its English wording, with `expr` (and `a`, `b` for `question.equal`, `property` for `question.property`) as parameters. A question in another wording, as from a Maxima running in another language, becomes `question.other` with the whole question as `question`.

Consecutive `rat: replaced` lines become one note listing each replacement, so a polynomial with ten decimals shows one line, not ten.

`note.zero_power` applies only when the cell still has a result; when the cell fails, the same line is the error (`error.division_by_zero`).

### 4. Lisp errors keep the whole condition

The parser treats "Maxima encountered a Lisp error:" through "To enable the Lisp debugger set *debugger-hook* to nil." as one error, drops the two SBCL lines, and joins the condition's lines with single spaces: "Maxima encountered a Lisp error: The value 1 is not of type LIST when binding LIST". The kernel-events overlay applies the same clean-up to error envelope text. `error.lisp_type` names the value and the type in words (`LIST` becomes "list"); `error.recursion` covers "Control stack exhausted"; `error.lisp` keeps the condition text for anything else.

### 5. Every failure leaves an output

`evaluate_cell` turns `AppError::Timeout` and `AppError::EvalCancelled` into an error output on the cell (`error.timeout` with `seconds`, `error.cancelled`), as it does for Maxima errors, and still returns the error to the caller. The window then shows the explanation in the cell; the log keeps its entry.

### 6. Maxima's debugger hint goes

Both paths drop the line ` -- an error. To debug this try: debugmode(true);` from error text. The parser still uses it to detect an error; it just doesn't keep it.

### 7. The window shows notes in place

`CellOutput.tsx` splits `textOutput` into lines and replaces each note's range with a note line: an info mark, the note's text, and a toggle that shows Maxima's lines instead. Text output moves before plots, so print text and notes appear in the order Maxima printed them.

## What changes for MCP clients and saved notebooks

- `run_cell` and `run_all_cells` return a question as a cell error with `is_error: true` and the question in `error`. A client that matched "Maxima needs more information" in a tool error must look at the result instead.
- `get_cell` outputs gain `notes`, and `error_info` gains `id` and `params`. Existing fields keep their meaning.
- Files keep their format. A question saves as an `error` output whose `evalue` starts "Maxima needs to know:". Error text in new files no longer has the debugger hint.

## Risks / Trade-offs

- [Maxima renames `retrieve`] → the init line skips the override when the function is missing, and the Maxima-backed tests fail on that version, so the release notes catch it.
- [A question inside `errcatch` now returns `[]` where it used to get an automatic `positive`] → that answer was a guess the user never saw; an error they can see is better. The changelog says so.
- [Templates match English wording] → a Maxima in another language gives `question.other` and unexplained notes, which still show Maxima's text.
- [Notes misread print output that looks like a note] → recognisers anchor on Maxima's exact prefixes (`rat: replaced`, `solve: `) and line starts; Maxima's lines stay one click away.
