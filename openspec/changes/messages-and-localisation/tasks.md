Each numbered group is one pull request, landing in order unless a group says otherwise. Each group runs `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, the Maxima-backed tests (`cargo test -p aximar-core -- --ignored`, `cargo test -p aximar-mcp -- --ignored`), `npm test` and `npm run build`, and updates the Unreleased section of `CHANGELOG.md` for changes MCP clients or users can see. A group that changes text runs `scripts/i18n-extract` and commits the template.

## 1. The catalog pipeline

- [x] 1.1 Add Lingui (`@lingui/core`, `@lingui/react`, `@lingui/vite-plugin`, `@lingui/cli`, `@lingui/format-po-gettext`) with `lingui.config` pointing at `locales/`, an `I18nProvider` around the app, and English as the only language; verify `npm run build` and that the window renders unchanged
- [x] 1.2 Add the `msg!` and `msg_ctx!` macros and the `Message` type (`id`, `template`, `params`, `text`) in aximar-core; verify with unit tests that `text` renders the template with its parameters and that a missing parameter fails the test build
- [x] 1.3 Add `scripts/i18n-extract` (Lingui template, `xgettext` over the Rust crates with `--keyword='msg!' --keyword='msg_ctx!:1c,2' --add-comments=i18n:`, `msgcat`, `msgmerge --previous` per language) and its `--check` mode; verify it produces `locales/messages.pot` with a test `msg!` string, its `// i18n:` note and a window string
- [x] 1.4 Add the CI job: GNU gettext from packages, `scripts/i18n-extract --check`, `msgfmt -c` on every `.po`, and a test comparing placeholder names and `max-length:N` flags between msgids and translations; verify the job fails on a stale template and on a `.po` with a renamed placeholder (the "catalog is checked" scenarios), then passes
- [x] 1.5 Add Lingui's pseudo-locale for development builds; verify by running the window in it that only text outside the catalog stays plain English

## 2. Message IDs, and errors.rs onto the catalog

- [x] 2.1 `ErrorInfo` gains `id` and `params`; its fields become `Message`s, serialised with their English `text`; verify existing MCP result tests pass unchanged apart from the new fields
- [x] 2.2 Move every `errors.rs` pattern onto `msg_ctx!` with its ID (`error.division_by_zero.title`, and so on); `error.arg_count` names the function, its definition and the arguments found, and stops suggesting `describe` for user functions; verify each existing `errors.rs` test passes with the same English text, plus the "Parameters fill the text" scenario
- [x] 2.3 Add the syntax-error IDs (`error.syntax_missing`, `error.incomplete_input`, `error.syntax_operator`, `error.syntax` for the rest) matching the parser's English patterns; verify with parser tests and Maxima-backed tests for `f(x;` and `1+;`
- [x] 2.4 Generate `docs/maxima-messages.md` from the template (each ID, the Maxima text it covers from its `// i18n:` note, its parameters and its English text) with a check mode in CI; add the test that every ID the code can produce is in the template; verify "The catalog and the code agree"

## 3. The language setting and switching

- [x] 3.1 Add the language setting to the app config, defaulting from `sys-locale` matched to the available catalogs with `fluent-langneg`; verify with unit tests for `es_CO` → `es` and `ja` → English (the two default scenarios)
- [x] 3.2 Add a language picker to Settings and switch the window with `i18n.load` and `i18n.activate`; the window translates `Message`s by template and context, falling back to `text`; verify with vitest tests that a switch re-renders an existing error explanation, and that a missing translation shows English ("Missing translations fall back to English")
- [x] 3.3 Build the native menus from the catalog with `rspolib`, rebuilt with `app.set_menu` on a switch; verify with a unit test that the Spanish catalog's menu labels come through, and by hand that the menu bar changes on a switch (the unit test is `menu_labels_come_from_the_catalog` in `menu.rs`; the menu bar following a catalog checked by hand through the translation preview, 28 November 2026)
- [x] 3.4 Pass the language to sessions: set `ax__language` at session start from the setting, and in each session on a switch (when idle, or when its evaluation ends); remove `AXIMAR_LANGUAGE`; verify with a Maxima-backed test that switching to Spanish then running `diff();` gives the Spanish message, and that English gives none ("Maxima's errors follow the switch")
- [x] 3.5 Verify "MCP stays in English" with an aximar-mcp Maxima-backed test in connected mode with the window set to Spanish

## 4. Window text and menus into the catalog

- [x] 4.1 Move the text of every component in `src/components` into Lingui, with a `comment` for each string and `context` where needed; verify with the pseudo-locale that no window text stays outside the catalog, and that `npm test` passes
- [x] 4.2 Move `menu.rs`'s labels into `msg!` with notes and `max-length` flags; verify with the menu unit test from 3.3
- [x] 4.3 Verify "Computations are never translated": vitest tests that results and `print` text render unchanged in Spanish, and a Maxima-backed test that a notebook saved in English and in Spanish gives identical files apart from timings

## 5. Questions become errors

- [x] 5.1 Override `retrieve` at session start, guarded by `fboundp`; delete `ASSUMPTION_QUESTION_RE`, `assumption_answer`, `drain_answering_questions`, the question branches in both readers and `AppError::AssumptionRequired`; verify with Maxima-backed tests (aximar-core, both paths) for the sign, yes/no, leak and `errcatch` scenarios
- [x] 5.2 Add the `question.*` IDs, mapping each question template to its ID and parameters, with `question.other` for anything else; verify with unit tests per template and one in another wording
- [x] 5.3 MCP reports a question as a cell error; verify with an aximar-mcp Maxima-backed test for the MCP scenario; check that the DAP server's tests still pass without its question handling

## 6. Every failure leaves an output, and Lisp errors keep their message

- [x] 6.1 `evaluate_cell` sets an error output for timeouts and cancellations (`error.timeout`, `error.cancelled`); verify with Maxima-backed tests for both scenarios, and a vitest test that the window shows the cell's error output
- [x] 6.2 Keep the whole Lisp condition on both paths, without SBCL's two lines; add `error.lisp_type`, `error.recursion`, `error.float_overflow` and `error.lisp`; verify with parser tests from captured SBCL output and Maxima-backed tests for both Lisp scenarios
- [x] 6.3 Drop the debugger hint from error text on both paths; verify with parser tests and the divergent-integral scenario
- [x] 6.4 Show the explanation, Maxima's translation and "Show original" in `EnhancedErrorOutput` in the user's language; verify with vitest tests for a translated error, an untranslated one and English ("Maxima's errors show in the user's language")

## 7. Notes

- [x] 7.1 Add `messages::notes` with the `note.*` IDs from the survey, `CellOutput.notes` filled after evaluation and on load, never written to files; verify with unit tests per note (including consecutive `rat` lines, print text that looks like a note, and `note.zero_power` only with a result) and the reopened-note scenario
- [x] 7.2 Show notes in the window as note lines in the user's language with a toggle for Maxima's lines, and text output before plots; verify with vitest tests of the note line and toggle, and by hand with the survey's cells
- [x] 7.3 Add the survey's other errors (`error.log_zero`, `error.part_index`, `error.part_atom`, `error.assign_target`, `error.empty_list`, `error.not_polynomial`, `error.algsys_failed`) to the catalog and docs; verify with unit tests from the survey's text

## 8. Translator tools and the Spanish catalog

- [x] 8.1 Add the preview loader, in every build: a Tauri command reading a `.po`, run-time compilation in the window, a file watcher, and the menus following it; verify with a vitest test of compiling a loaded catalog and by hand with the "Live preview" scenario (checked by hand on macOS, November 2026)
- [x] 8.2 Add show-sources mode, in every build; verify with a vitest test that pointing at a string shows its source text, note and file
- [x] 8.3 Set up Storybook (React with Vite), with a shared decorator that mocks Tauri's commands and a toolbar that switches language through Lingui, the pseudo-locale included. Add stories for the states that are hard to reach in a running app: each kind of error explanation, notes, questions, a timeout, the trust banner, Settings and the other dialogs. With Playwright over the static Storybook build, take a screenshot of every story in every language; in a tagging mode, record which entries each story renders and write `#. screen: <story>` comments into the template. Publish the static Storybook from CI as the gallery. Verify that every story has a screenshot in each language, and that the template gains screen comments
- [x] 8.4 Create `locales/es.po` with `msginit` and `msgmerge --compendium` against Maxima's `es.po`, plus `locales/glossary.tbx` seeded from Maxima's Spanish terms and `locales/README.md` (tone, capitalisation, do-not-translate list, decimal points in code, how to preview); verify `msgfmt -c` passes and Spanish appears in the language picker

## 9. Help in the user's language

- [x] 9.1 Give catalog-gen a language option reading `doc/info/<lang>/maxima.texi`, and generate `core-doc-summaries.es.json` (summaries only, through mxpm); verify the Spanish index has an entry for `integrate` with Spanish text
- [x] 9.2 Look summaries up in the user's language, then English, and index both for search; verify with unit tests for the "Spanish summary" and "English fallback" scenarios and a search that finds a symbol by its Spanish summary

## 10. Close-out

- [ ] 10.1 Check every scenario in the `maxima-messages` and `localisation` specs against the tests; run the survey cells in the window in English and Spanish and over MCP; update `docs/i18n.md`'s statuses and roadmap; update `CHANGELOG.md`; archive the change
