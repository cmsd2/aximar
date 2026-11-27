## Context

`docs/i18n.md` is the strategy this change implements: where each kind of text comes from, where its translations come from, the data flow and the fragility table. This design covers how.

**Already in place.** #107 starts Maxima with `LANGUAGE=en` on every backend, so Aximar's English matching holds in any environment (`tests/spanish_locale.rs`). #108 added `maxima/ax_messages.lisp`, which wraps `errormsg()` to print Maxima's own translation of each error on a tagged line, and `maxima/messages.rs`, which loads it and parses the line into `error_translated`. The language comes from `AXIMAR_LANGUAGE`, a stand-in.

**Errors.** `maxima/errors.rs` matches a raw error against a dozen patterns and builds an `ErrorInfo` (title, explanation, suggestion, example) in English. The legacy parser collects error lines into `EvalResult.error`; the kernel-events overlay takes them from error envelopes. The window shows `error_info` in `EnhancedErrorOutput`, with Maxima's text behind "Show raw error".

**Questions.** Maxima asks through one Lisp function, `retrieve` (`src/macsys.lisp`), which prints the question to `*query-io*` and reads an answer with `mread`. Its callers: `ensure-sign` (sign questions, five forms), `$askequal` ("Is a equal to b?"), `ask-prop` ("Is n an integer?"), and a rare one in `hayat.lisp`. Each loops until it gets a valid answer.

Aximar writes each cell as one block: the expression, then housekeeping lines (the one-line form and `tex(%)`, the label print, the end sentinel). `process.rs` matches sign questions with `ASSUMPTION_QUESTION_RE`, writes `positive;` and the sentinel again, drains until the sentinel, and returns `AppError::AssumptionRequired`. Maxima reads the queued housekeeping lines first, as invalid answers, and asks again; each repeat gets another `positive;`, and the spares run as expressions in the next cell. Yes/no questions don't match the pattern, so Maxima re-asks until the timeout interrupts it. `read_dap_response` repeats the same logic for the debugger.

**Notes.** Anything Maxima prints that isn't an error, a label or LaTeX goes into `text_output`, which the window shows as monospace text after any plot and before the result.

**Lisp errors.** SBCL prints "Maxima encountered a Lisp error:", a blank line, the condition over several lines, a blank line, and "Automatically continuing. / To enable the Lisp debugger set *debugger-hook* to nil." The parser ends an error at the first blank line.

**Window text.** About 4,600 lines of React components hold their text as literals, and `src-tauri/src/menu.rs` builds the native menus from literals. There is no catalog, no language setting and no i18n library.

### Survey

Running likely triggers in Maxima 5.50 (SBCL) gave these messages, which set the first catalog entries:

| Kind | Maxima prints | ID |
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
| Syntax | incorrect syntax: Missing ) / Missing ] | `error.syntax_missing` |
| Syntax | incorrect syntax: Premature termination of input at ;. | `error.incomplete_input` |
| Syntax | incorrect syntax: x is not an infix operator / not a prefix operator | `error.syntax_operator` |

The existing `errors.rs` patterns keep their meaning and get IDs: `error.division_by_zero`, `error.syntax`, `error.incomplete_input`, `error.arg_count`, `error.undefined_variable`, `error.unknown_function`, `error.divergent_integral`, `error.inconsistent_equations`, `error.matrix_dimension`, `error.package_not_found`, `error.lisp`. Timeouts and cancellations get `error.timeout` and `error.cancelled`; a question Aximar can't classify gets `question.other`.

## Goals / Non-Goals

**Goals:**
- One catalog, one format and one extraction step for all of Aximar's text, Rust and React alike.
- No question ever hangs a cell or leaks into another; every failure leaves an output in its cell.
- Every friendly message addressable by a stable ID, and renderable in any language without parsing English Aximar produced.
- A translator can work from the `.po` file alone, and see their work in the app as they save.

**Non-Goals:**
- Translating text inside aximar-core: core renders English, and only the window and the menus translate.
- Changing how Maxima runs or what kernel-events sends.

## Decisions

### 1. gettext `.po` with English msgids, and IDs as `msgctxt`

The catalog is `locales/messages.pot` with `locales/<lang>.po`, in gettext's format, because Maxima's translations are `.po` too: a translator uses one editor for both, and Maxima's `es.po` can suggest translations.

Window text uses the English as the msgid, with `msgctxt` only where one English string means two things. Friendly messages use their stable ID as `msgctxt` and the English template as the msgid, one entry per field: `error.division_by_zero.title`, `error.division_by_zero.explanation`, and so on. Translators see English in their editor; code and MCP clients see IDs that survive rewording; and when the English changes, `msgmerge` marks the entry fuzzy with the old English beside it instead of losing the translation.

Placeholders are named, in braces (`{expr}`), which Lingui, Rust format strings and `msgfmt -c` all accept. Friendly messages use no ICU plurals or selects, so both extractors and `msgfmt` read them the same way. Window text may use plurals, which the `po-gettext` formatter stores as native `msgid_plural` entries.

*Alternatives:* `friendly-maxima-messages`' JSON table with an ID per entry (no translator tooling, a second format beside Maxima's); IDs as msgids (translators would see `error.division_by_zero.title` in their editor instead of English); Fluent (comments and grammar handled well, but neither Maxima nor the desktop `.po` editors use it).

### 2. Core renders English; the window renders the user's language

```rust
pub struct Message {
    pub id: String,                       // "question.sign", the msgctxt
    pub template: Cow<'static, str>,      // "Maxima needs to know the sign of {expr}", the msgid
    pub params: BTreeMap<String, String>, // {"expr": "a"}
    pub text: String,                     // the English, rendered from template and params
}
```

`msg_ctx!("question.sign", "Maxima needs to know the sign of {expr}", expr = e)` builds one. `ErrorInfo` gains `id`; its `title`, `explanation` and `suggestion` become `Message`s, each still serialised as its English `text`, so MCP clients read them as they do now. Beside them the JSON carries `params` (every message's parameters) and `messages` (each field's context and English template), which is what the window needs to look a field up. `example` stays a string: it's Maxima code, which no language changes. A field with variants has a context per variant (`error.arg_count.explanation.too_few`). Notes are `Message`s plus the range of `text_output` lines they cover.

The window translates a `Message` with Lingui, looking up the template under the ID's context and filling in the parameters, and falls back to `text`. aximar-core never needs the user's language, so the MCP server, batch runs and the window share one code path, and switching language re-renders existing outputs without re-running anything.

*Alternative:* core renders in the user's language. Every adapter would need the setting, MCP would follow the window's language, and a switch would need every output recomputed.

### 3. Extraction: Lingui for React, `xgettext` for Rust, one template

React text goes through Lingui's `t` macro and `<Trans>`, with `comment` (becoming `#.`) and `context` (becoming `msgctxt`). Rust text goes through `msg!` and `msg_ctx!`, with translator notes in a `// i18n:` comment above the call. `scripts/i18n-extract`:
1. runs `lingui extract-template`;
2. runs `xgettext --language=Rust --keyword='msg!' --keyword='msg_ctx!:1c,2' --add-comments=i18n:` over the Rust crates;
3. joins the two with `msgcat` into `locales/messages.pot`;
4. runs `msgmerge --previous` into each `locales/<lang>.po`, then `msgmerge --compendium` with Maxima's catalog for that language to suggest translations for new entries.

Tested with `xgettext` 1.0: it needs the `!` in the keyword, reads notes only from source comments (not macro arguments), and marks placeholder strings `rust-format`, so `msgfmt -c` checks them.

*Alternative:* a custom Lingui extractor for `.rs` files. One fewer tool, but a hand-written Rust parser instead of GNU's.

### 4. Loading and switching

- **Window:** the Lingui Vite plugin compiles each `.po` on import, and the window loads a language when it's chosen. Switching calls `i18n.load` and `i18n.activate`; React re-renders.
- **Menus:** `src-tauri` embeds each `.po` with `include_str!`, parses it with `rspolib`, and rebuilds the menu with `app.set_menu` on a switch. `tr`'s `po-translator` evaluates `Plural-Forms` if a menu ever needs a plural.
- **The setting** lives in the app config beside the backend settings. The default comes from `sys-locale`, matched to the available catalogs with `fluent-langneg` (`es_CO` to `es`).
- **Maxima:** a session's start sets `ax__language` from the setting, replacing `AXIMAR_LANGUAGE`. A switch sets it again in each idle session with one evaluation, and in a busy one when its evaluation ends.
- **Preview builds:** a Tauri command reads a `.po` from disk and returns its entries; the window compiles them at run time, and a file watcher sends them again on each save. The same entries replace the menus' catalog.

### 5. Questions raise an error inside Maxima

Each Maxima process loads `ax_questions.lisp` as a `:lisp` line when it starts, before kernel-events. It replaces `retrieve`, through which Maxima asks every question, and wraps `toplevel-macsyma-eval` with a catch. `retrieve` throws the question to that catch, which raises "Maxima needs to know: <question>" as an ordinary Maxima error at the top of the statement.

The throw matters. A question isn't an error in Maxima: `errcatch` doesn't stop it reaching the user, and library code relies on that. `laplace` runs `defint` inside `errset` and falls back to another method on an error, so a question raised as an error inside it would be swallowed and the result would change silently. A throw to Aximar's own tag passes every `errcatch` and `errset` (their cleanup still runs, so `laplace`'s assumptions go), and is caught only around the whole statement. The whole question goes in the error's pattern, with no arguments, because kernel-events reports an error's pattern without them. In the debugger, `break-dbm-loop` turns the catch off, so a question there is an error where it's asked.

The cell's housekeeping lines run as usual, the end sentinel arrives, and the cell reports an error. Nothing is written to stdin after the cell, so nothing leaks. The override applies to every Maxima `MaximaProcess` starts: the window, MCP, batch runs and the DAP server.

`ASSUMPTION_QUESTION_RE`, `assumption_answer`, `drain_answering_questions`, the question branches of both readers, and `AppError::AssumptionRequired` go.

*Alternatives:* answer questions automatically (what Aximar does now, and the cause of the leak); answer interactively in the cell (a non-goal: MCP and batch runs can't, and the answer isn't in the notebook); send housekeeping lines only after the expression finishes (fixes the leak, not the yes/no hang, and doubles the round trips).

### 6. Recognition happens in aximar-core, on both paths and on load

`messages::explain_error(raw, catalog, packages) -> Option<ErrorInfo>` replaces `enhance_error_with_packages`, and `messages::notes(text_output) -> Vec<Note>` finds notes. `CellOutput` gains `notes`, filled from an evaluation and by `notebooks::format::explain_errors` (renamed `explain`) when a file is opened. Notes, explanations and translations are never written to files.

Each question template maps to an ID by its English wording, with `expr` (and `a`, `b` for `question.equal`, `property` for `question.property`) as parameters; anything else becomes `question.other`. Recognisers match English only, which the English pin guarantees.

Consecutive `rat: replaced` lines become one note listing each replacement. `note.zero_power` applies only when the cell still has a result; when the cell fails, the same line is the error (`error.division_by_zero`).

Syntax errors match the parser's English patterns after `incorrect syntax: `, with the delimiter or operator as a parameter.

### 7. Lisp errors keep the whole condition

The parser treats "Maxima encountered a Lisp error:" through "To enable the Lisp debugger set *debugger-hook* to nil." as one error, drops the two SBCL lines, and joins the condition's lines with single spaces. The kernel-events overlay applies the same clean-up to error envelope text. `error.lisp_type` names the value and the type in words (`LIST` becomes "list"); `error.recursion` covers "Control stack exhausted"; `error.lisp` keeps the condition text for anything else.

### 8. Every failure leaves an output, without the debugger hint

`evaluate_cell` turns `AppError::Timeout` and `AppError::EvalCancelled` into an error output on the cell (`error.timeout` with `seconds`, `error.cancelled`), as it does for Maxima errors, and still returns the error to the caller.

Both paths drop ` -- an error. To debug this try: debugmode(true);` from error text. The parser still uses it to detect an error; it just doesn't keep it.

### 9. The window shows notes and translated errors in place

`CellOutput.tsx` splits `textOutput` into lines and replaces each note's range with a note line: an info mark, the note's text in the user's language, and a toggle that shows Maxima's lines instead. Text output moves before plots, so print text and notes appear in the order Maxima printed them.

`EnhancedErrorOutput` shows the explanation in the user's language, then Maxima's translated message (`error_translated`) when there is one, with Maxima's English behind "Show original".

### 10. Translator tools

- **Show sources:** in preview builds, holding a modifier and pointing at text shows its msgid, context, note and source file. Lingui's message descriptors carry all four in development; the preview build keeps them.
- **Screenshots:** a Playwright script runs the window through Vite with Tauri's commands mocked, visits each screen in each language and saves a screenshot. In a tagging mode each translated string renders with its message ID in a data attribute, so the script records which entries appear on which screen and writes `#. screen: <name>` comments into the template. CI publishes the screenshots as a gallery.
- **Glossary and guide:** `locales/glossary.tbx`, seeded from Maxima's Spanish terms, and `locales/README.md` with tone, capitalisation, the do-not-translate list and how to use the preview.
- **Seeded `es.po`:** `msginit` plus `msgmerge --compendium` against Maxima's `es.po`, so matching strings arrive as fuzzy suggestions.

### 11. Help per language

catalog-gen takes a language and reads `doc/info/<lang>/maxima.texi`, writing `core-doc-index.<lang>.json`. Aximar embeds the Spanish index beside the English one and looks a symbol up in the user's language first, then in English. The search index covers both languages' summaries.

### 12. CI checks

- `scripts/i18n-extract --check` fails when the template differs from what extraction produces.
- `msgfmt -c` on every `.po`.
- A test compares placeholder names between each msgid and its translations, and checks `max-length:N` flags in comments.
- The message-ID test from decision 2, and a check that `docs/maxima-messages.md` matches its generator.

CI installs GNU gettext from the distribution's packages (0.24 or later).

## What stays in aximar-core and what the adapters keep

- **aximar-core:** recognising errors, notes and questions; building `Message`s in English; the `retrieve` override and the `errormsg()` wrapper; setting `ax__language` in a session; the help indexes and lookup with fallback.
- **Tauri:** the language setting and its default, menus from the catalog, the preview loader and file watcher, and passing the language to sessions.
- **MCP:** unchanged behaviour apart from the result shapes; text stays English.
- **Window:** all translation of window text and `Message`s, notes in place, the preview and show-sources modes.

## What changes for MCP clients and saved notebooks

- `run_cell` and `run_all_cells` return a question as a cell error with `is_error: true` and the question in `error`. A client that matched "Maxima needs more information" in a tool error must look at the result instead.
- `get_cell` outputs gain `notes`, and `error_info` gains `id` and `params`. Existing fields keep their meaning and their English text.
- Files keep their format. A question saves as an `error` output whose `evalue` starts "Maxima needs to know:". Error text in new files no longer has the debugger hint. No translation is ever written to a file.

## Risks / Trade-offs

- [Two extractors, two toolchains] → one script runs both, and CI fails when the template is stale; GNU gettext becomes a developer dependency only for those who change text.
- [Lingui's run-time lookup by message and context must match the template exactly] → the `Message` carries the exact template string from the `msg_ctx!` call, and a test renders every Rust message through the window's lookup.
- [Maxima renames `retrieve`] → the init line skips the override when the function is missing, and the Maxima-backed tests fail on that version.
- [A question inside `errcatch` now stops the statement, where it used to get an automatic `positive`] → that answer was a guess the user never saw. Stopping matches Maxima, where `errcatch` doesn't stop a question reaching the user; letting `errcatch` swallow it would send user code and library functions such as `laplace` down their error paths without a word. The changelog says so.
- [Maxima renames `toplevel-macsyma-eval` or `break-dbm-loop`] → the override skips what's missing, and the Maxima-backed tests fail on that version.
- [Notes misread print output that looks like a note] → recognisers anchor on Maxima's exact prefixes and line starts; Maxima's lines stay one click away.
- [Window text moved into the catalog by hand misses strings] → the pseudo-locale run shows any text that didn't go through the catalog, and the screenshot gallery covers each screen.
- [Rewording English invalidates translations] → `msgmerge` keeps the old translation as fuzzy with the old English, so translators review instead of starting again.
- [Maxima's catalog missing on some backends] → English only, as today; a later change can report which languages the installed Maxima has.

## Migration Plan

Each task group lands on its own; the window keeps working in English throughout. Translations arrive as `es.po` fills in, and the language setting lists Spanish once the catalog exists, falling back to English per message. `AXIMAR_LANGUAGE` stops working when the setting lands; it was never documented for users. Rollback is per pull request.
