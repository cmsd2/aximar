## Why

Aximar's help is English for anyone with full documentation installed. The messages-and-localisation change embedded Spanish summaries from Maxima's Spanish manual, but the docs panel shows a symbol's full documentation whenever it has some, and the `maxima-core-docs` package gives 2,705 core symbols full English documentation. A Spanish user with that package, the usual setup, never sees the Spanish. Beneath that, a doc index has no notion of language: mxpm generates core docs in English only, and catalog-gen reaches the Spanish manual by copying Maxima's source tree and swapping `doc/info` for `doc/info/es`.

Maxima's manual exists in Spanish, Portuguese, Brazilian Portuguese, German, Japanese and Russian, at different levels of completeness. A student in the pilot that #44 asked for should read the help in Spanish where the manual has it, and see plainly where it falls back to English.

## What Changes

- **Doc indexes carry a language.** A doc index gains a `language` field (default `en`, so every existing file keeps working), and a translated index is named `<stem>-doc-index.<lang>.json`, beside the English one.
- **Language packs for the core docs.** `mxpm doc generate-core-docs --lang <lang>` builds from `doc/info/<lang>`, and core-docs publishes `maxima-core-docs-es`, `-pt`, `-pt-br`, `-de`, `-ja` and `-ru` to the package index, each pinned to the same Maxima release as `maxima-core-docs`. Any package may ship translated indexes under the same naming.
- **The catalog keeps every language** (aximar-core). It holds each symbol's entries per language and answers in a language chain, such as Spanish then English, symbol by symbol. The embedded Spanish summaries become a summary-only Spanish layer, so the help works before any pack is installed.
- **The docs panel says what it shows.** It shows the chosen language where there is a translation. Where a symbol falls back to English, it says so, and where a translation's signatures differ from the English it notes that the translation may be out of date. Search matches either language and shows results in the chosen one.
- **A choice of documentation language**, in the docs panel's header: "Same as Aximar", which is the default, or one language. The menu lists the languages with installed documentation. A language without its pack shows the `mxpm install` command that adds it.
- **Every front end.** MCP's `get_function_docs` and `search_functions` take an optional `language`. maxima-lsp answers hover and completion in the editor's language, from the client's `locale`.
- MCP results gain a `language` field naming the language of each text. Calls without `language` answer in English, as now, so nothing existing breaks.

## Capabilities

### New Capabilities

- `documentation-languages`: how documentation records its language, how front ends choose and fall back between languages, what they tell the reader, and how a language's documentation gets installed.

### Modified Capabilities

None. No existing spec covers the documentation catalog or the docs panel.

## Impact

- **Core** (`crates/aximar-core/src/catalog/`): the doc index format, the per-language store, the language chain in queries, the embedded summaries as a layer.
- **Tauri**: `commands/catalog.rs` passes the chain instead of patching summaries in (`i18n::help_summary` goes).
- **Frontend**: `DocsPanel.tsx` (language menu, fallback and stale notes), its strings in the catalog, a story per state.
- **MCP**: an optional `language` on `get_function_docs` and `search_functions`; a `language` field in results.
- **maxima-lsp**: reads `locale` from `initialize`.
- **Other repositories**: mxpm (`--lang` and the `language` field), core-docs (building and publishing the language packs), package-index (their entries). Each lands as its own PR there; this change records what each must do.
- **tools/catalog-gen**: `summaries` uses `mxpm doc generate-core-docs --lang` instead of copying Maxima's source tree.

## Non-goals

- Translating the manual. The packs are Maxima's own translations, as complete and as current as Maxima's translators have made them.
- Installing packages from inside Aximar. The panel shows the mxpm command; an install button needs Aximar to find and run mxpm, which is its own change.
- Making Maxima's own `describe` and `?` follow the language. Aximar starts Maxima with `LANGUAGE=en` so it can read Maxima's errors; changing that belongs with the messages work.
- Translated documentation for third-party packages. The format allows it; writing them is up to each package.
