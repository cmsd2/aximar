## Why

A professor in Colombia wants Aximar in Spanish for a student pilot next semester (#44), and offers to translate. Aximar's text is English strings spread through React and Rust, so there is nothing to hand a translator yet.

The messages Aximar shows for Maxima's output need work before they can be translated:
- **Questions hang or leak.** Aximar recognises only sign questions, answers them behind the cell's housekeeping lines (so spare answers run in the next cell), and lets yes/no questions wait for the 30-second timeout.
- **Timeouts and questions leave the cell empty.** The message goes only to the log panel.
- **Lisp errors lose their cause**, because the parser stops at the blank line SBCL prints before the condition.
- **Notes read like print output**, such as `rat: replaced 0.5 by 1/2 = 0.5`, and some mislead.

This change was planned as `friendly-maxima-messages` (no tasks done), which gave every message a stable ID and an English table ready for translation. Its table now moves into the gettext catalog that `docs/i18n.md` plans, so both land as one change. #107 (Maxima in English) and #108 (Maxima's own errors in the user's language) are its groundwork.

## What Changes

**Catalog and language**
- **One gettext catalog**, `locales/messages.pot` with a `locales/<lang>.po` per language, for all of Aximar's text: the window, the native menus, and every friendly message. English text is the msgid. Translator notes, source references and length limits travel in the file.
- **The window uses Lingui**; Rust marks strings with `msg!` and `msg_ctx!` macros, and `xgettext` extracts them. One script builds the template and merges each language, with Maxima's own catalog suggesting translations for new strings.
- **A language setting**, defaulting to the system language. Changing it switches the window, the menus, Maxima's translated errors and the help without a restart. The `AXIMAR_LANGUAGE` variable from #108 goes.
- **Spanish help summaries**: catalog-gen builds an index from Maxima's Spanish manual, with English for symbols it lacks.
- **Translator tools**: a pre-release setting that loads a `.po` from disk and reloads it on save, a show-sources mode, a Storybook gallery of the window's states in every language, its screenshots tagged with their strings, a glossary and a translator guide. An `es.po` seeded from Maxima's Spanish is ready for #44's translator.
- **CI checks** that the template is current, that `msgfmt -c` passes, and that translations keep their placeholders and length limits.

**Friendly messages** (from `friendly-maxima-messages`)
- **Every friendly message has a stable ID and named parameters**, such as `question.sign` with `expr: a`. The ID is the entry's `msgctxt` in the catalog. Errors and notes carry the ID, parameters and English text, so the window renders them in the user's language and nothing parses English Aximar produced. `docs/maxima-messages.md` lists every ID, generated from the template.
- **Questions become errors.** At session start Aximar replaces Maxima's `retrieve`, the one function every question goes through, with one that raises a Maxima error quoting the question. Every kind of question stops the cell cleanly, with an explanation naming the line that answers it, such as `assume(a > 0)`. The answering and draining code in `process.rs` and `AppError::AssumptionRequired` go.
- **Timeouts and cancellations leave an error output in the cell.**
- **Lisp errors keep their message**, without SBCL's "Automatically continuing" lines, and common ones get explanations.
- **Notes become friendly note lines**, with Maxima's text a click away. Text output shows before plots.
- **Maxima's syntax errors** get explanations from the catalog, since Maxima doesn't translate them.
- **Error text drops Maxima's debugger hint**, ` -- an error. To debug this try: debugmode(true);`.
- **BREAKING (MCP):** `run_cell` and `run_all_cells` report a question as a cell error (`is_error: true`) instead of failing with "Maxima needs more information". `get_cell` gains `notes`, and `error_info` gains `id` and `params`.

## Capabilities

### New Capabilities

- `maxima-messages`: how Aximar shows Maxima's errors, notes and questions: message IDs and parameters, questions as errors, Lisp errors, notes, timeouts, and loaded outputs.
- `localisation`: the language setting and switching, the catalog and its tooling, Maxima kept in English with its errors shown in the user's language, help in the user's language, translator tools, and the CI checks.

### Modified Capabilities

None. `cell-execution` describes statuses and trust, which this change keeps.

## Impact

- **Core:** `maxima/process.rs` (questions, init), `maxima/errors.rs` (onto `msg_ctx!` and IDs), `maxima/messages.rs` (from #108, grows the `Message` type and recognisers), `maxima/legacy/parser.rs`, `maxima/envelope/`, `evaluation.rs`, `notebook.rs`, `notebooks/format.rs`, `session_ops.rs` (language), the catalog's help index.
- **MCP:** `server.rs` result shapes (see BREAKING). MCP text stays English.
- **Tauri:** `menu.rs` (menus from the catalog, rebuilt on switch), `commands/config.rs` (the language setting), a command to load a `.po` from disk.
- **Frontend:** Lingui throughout `src/components`, `EnhancedErrorOutput.tsx`, a note line component, Settings (language, preview loader), show-sources mode.
- **Tools:** `tools/catalog-gen` (one index per language), new `scripts/i18n-*` scripts, Storybook and its stories, CI jobs.
- **Dependencies:** `@lingui/core`, `@lingui/react`, `@lingui/vite-plugin`, `@lingui/cli`, `@lingui/format-po-gettext`; `rspolib` and `tr` (with `po-translator`) in Rust; GNU gettext 0.24 or later in CI and for developers who extract.
- **The DAP server** loses its question handling along with `process.rs`'s.
- **Issues:** closes #44's localisation request; touches #56 (friendly errors reach the window, fixed in #82).
- **Saved notebooks:** unchanged in format. A question saves as an `error` output. Notes, explanations and translations aren't stored; Aximar works them out again when it opens a file.
- **Replaces** the `friendly-maxima-messages` change, which this change removes.

## Non-goals

- Writing the Spanish translations. The change ships an `es.po` seeded from Maxima's catalog; #44's contributors complete it.
- Translating MCP tool text, aximar-mcp's results, maxima-dap or the VS Code extension.
- Overriding Maxima's own translations in Aximar's catalog. Fixes to Maxima's messages and manual go upstream.
- Full help text in other languages: the `maxima-core-docs` package stays English.
- A hosted translation service. The repository is the source of truth; Weblate can come later on the same files.
- Answering questions interactively in the cell, and `read()` and `readonly()`, which read stdin directly.
- Explaining every message Maxima has. The catalog starts with the survey in `design.md` and grows.
- The kernel-events package. Nothing in it changes.
