## 1. A language-aware catalog (aximar, first PR)

- [x] 1.1 Add `language` to `DocIndex` (default `en`) and the `<stem>-doc-index.<lang>.json` naming to the loader, skipping a file whose name and field disagree; verify with core tests `an_index_without_a_language_is_english`, `a_translated_index_loads_by_name` and `a_mismatched_index_is_skipped`
- [x] 1.2 Keep entries per language in `DocIndexStore`, add `LanguageChain` and `Resolved`, and resolve `get`, `complete`, `signature_info` and `search` per symbol; verify with core tests `a_translated_symbol_comes_first`, `a_missing_translation_falls_back_to_english`, `differing_signatures_mark_a_translation_stale` and `search_matches_either_language`; the English-only methods stay as they were, for MCP and the LSP until group 3
- [x] 1.3 Load the embedded Spanish summaries as a summary-only layer, merged field by field, and remove `i18n::help_summary`, `set_help_language` and `summaries::localised`; verify with a core test `a_summary_layer_sits_above_english_documentation`, using a fixture full English index like `maxima-core-docs`'s
- [x] 1.4 Pass the chain from the Tauri catalog commands, built from the app's language; verify the existing catalog command tests, and a test that the window's `get_function` for `integrate` in Spanish returns the Spanish summary with the English body when a full English index is installed

## 2. The docs panel (aximar, one PR)

- [x] 2.1 Add `docs_language` to the config (`None` for "Same as Aximar") and the language menu to the docs panel's header, listing languages with documentation by their own names; verify with vitest tests of the menu's choices and that a choice persists through `set_config`
- [x] 2.1a Move the language menu from the docs panel's header to Settings > General as Help language, emit `docs-language-changed` so open docs panels show their page again, and link the panel's language notes to the setting; verify with vitest tests of the setting's choices, the refresh and the link
- [x] 2.2 Show the fallback note, the stale note with the English signatures, and the strip naming the missing pack with its `mxpm install` command and a copy button; verify with vitest tests for each, and Storybook stories for a translated symbol, a fallback, a stale translation and a missing pack, in English, Spanish and the pseudo-locale
- [x] 2.3 Extract the new strings with translator notes, and describe documentation languages in `docs/i18n.md`; verify `scripts/i18n-extract --check` and the catalog test pass

## 3. MCP and the language server (aximar, one PR)

- [x] 3.1 Add the optional `language` to `get_function_docs` and `search_functions`, with `language`, `fallback_from` and `stale` in results; verify with MCP server tests `docs_in_spanish`, `docs_without_a_language_are_english` and `search_in_spanish`
- [x] 3.2 Read `locale` from `initialize` in maxima-lsp and use it for hover, completion and signature help; verify with LSP tests that a `locale` of `es` gives Spanish hover text, and that a missing `locale` gives English

## 4. mxpm (mxpm repository, one PR)

- [x] 4.1 Add `--lang` to `doc generate-core-docs` and `doc build`, mapping `pt-BR` to `pt_BR`, adding the English tree's shared macro files to the include path, and writing `language` and the `.<lang>` file names; verify with mxpm tests that `--lang es` over a fixture tree writes `maxima-core-docs-doc-index.es.json` with `"language": "es"`, and that no `--lang` gives today's output byte for byte
- [x] 4.2 Move catalog-gen's `summaries` onto `mxpm doc generate-core-docs --lang` (back in aximar); verify the regenerated `core-doc-summaries.es.json` matches today's file

## 5. Language packs (core-docs and package-index, one PR each)

- [x] 5.1 Build `maxima-core-docs-<lang>` in core-docs' CI for each language covering at least half the English symbols, each depending on `maxima-core-docs` at the same version; verify the build prints each language's coverage and produces packs for the languages over the threshold
- [x] 5.2 Add the packs to the package index; verify `mxpm install maxima-core-docs-es` installs `~/.maxima/maxima-core-docs-es/doc/maxima-core-docs-doc-index.es.json`, and by hand that Aximar's docs panel then shows Spanish full documentation for `integrate` (checked by hand: `mxpm install maxima-core-docs-es`, then Spanish help for `integrate`, 28 November 2026)
