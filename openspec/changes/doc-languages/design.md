## Context

**The format.** A doc index (`DocIndex` in `crates/aximar-core/src/catalog/doc_index.rs`) is JSON: `version`, `package`, `source`, `symbols` (name to entry: type, signature and signatures, summary, `body_md`, examples, see-also, category, keywords) and `sections`. It has no language.

**Where indexes come from.**
- mxpm's `doc build` writes `<stem>-doc-index.json` beside a package's `.info` file and Lisp index. The `.info` and index serve Maxima's `describe`; the JSON serves Aximar.
- `doc generate-core-docs` turns Maxima's texinfo into the `maxima-core-docs` package: `makeinfo --xml`, then Markdown, then the same build. Texinfo stays the source, and the JSON is a layer over it that `.info` can't provide: per-symbol signatures, categories and examples, search, and one shape for the window, the LSP and MCP.

**How Aximar loads them.** `DocIndexStore::load` embeds a slim English core index (summaries, no bodies), then overlays every `~/.maxima/<package>/doc/*-doc-index.json`, preferring a full index over a slim one. A later entry for a symbol replaces an earlier one. The `maxima-core-docs` package's full index replaces 2,705 embedded entries with ones that carry `body_md`.

**Spanish today.** catalog-gen's `summaries` command runs `generate-core-docs` on a copy of Maxima's source whose `doc/info` is `doc/info/es`, and keeps each symbol's summary: 2,318 of 2,698 symbols, embedded as `core-doc-summaries.es.json`. The Tauri layer swaps a summary for its Spanish one (`i18n::help_summary`), but `symbol_to_function` shows `body_md` whenever it isn't empty. So with `maxima-core-docs` installed, the docs panel shows English. Completion and category lists use summaries, so they show Spanish.

**Maxima's manuals.** `doc/info/` has subdirectories `de` (89 texinfo files), `es` (87), `ja` (95), `pt` (75), `pt_BR` (79), `ru` (45), `zh` (3) and `fr` (1). Most were last updated years before the current English manual.

**Who reads the catalog.** The Tauri commands (`commands/catalog.rs`), aximar-mcp (`get_function_docs`, `search_functions`, `complete_function`), maxima-lsp (hover, completion, signature help), and catalog search.

## Goals / Non-Goals

**Goals:**

- One catalog that holds every language and answers in a chain, the same for every front end.
- A reader always knows which language they're reading, and when a translation may be stale.
- Language packs built by the existing pipeline, with no hand-made trees.
- Older indexes, packages and MCP clients keep working unchanged.

**Non-Goals:**

- Translating documentation, installing packages from Aximar, or changing Maxima's own `describe` (see the proposal).

## Decisions

### 1. The language lives in the file, and in its name

`DocIndex` gains `language: String`, defaulting to `"en"` through `#[serde(default)]`. A translated index is named `<stem>-doc-index.<lang>.json` (and `<stem>-slim-doc-index.<lang>.json`); the loader groups files by stem and language, so the full-over-slim preference holds per language. The name lets the loader see a file's language without parsing it, and the field lets a file stand alone, such as one an MCP client reads. A file whose name and field disagree is skipped with a warning, since either could be the mistake.

Tags are BCP 47, as `pt-BR`, matching the app's `language` setting. Maxima's directory `pt_BR` maps to `pt-BR` in mxpm.

Alternative: one file holding every language. A pack per language keeps downloads small and lets packages add languages one at a time. Rejected.

### 2. The store keeps languages apart, and resolves late

`DocIndexStore.symbols` becomes `name -> language -> (package, SymbolEntry)`. Ingesting an index replaces entries for its own language only. The BM25 index covers every language's summaries and keywords, with each indexed row tagged by language.

Queries take a `&LanguageChain`, a non-empty list of tags that always ends with `en`; `LanguageChain::new(["es"])` gives `es, en`. Each query resolves per symbol:

- `get(name, chain)` returns `Resolved { entry, language, fallback_from: Option<Lang>, stale: bool }`. `fallback_from` is set when the chain's first language has no entry. `stale` is set when a translated entry's signature set differs from the English entry's.
- A summary-only layer (decision 3) merges field by field: the summary from the first language that has one, `body_md` and the rest from the first language with a full entry, with a language reported for each.
- `search(query, chain)` ranks across every language, then shows each hit in the chain's first language that has it.
- `complete(prefix, chain)` and `signature_info(name, chain)` take the chain too, so the LSP and MCP get the same answers as the window.

This logic lives in aximar-core, so Tauri, MCP and the LSP only translate parameters and results. The store keeps its English-only methods (`get`, `search`, `complete`, `signature_info`, `hover_markdown`) beside the chain-aware ones (`resolve`, `search_in`, `complete_in`, `signature_info_in`, `hover_markdown_in`), so MCP and the LSP behave as before until group 3 moves them over. A function the window shows carries a `languages` field (the summary's and the body's language, `fallback_from`, `stale` and the English signatures), which the docs panel's notes read in group 2. Until then, a translated summary above English documentation shows as the summary, a blank line, then the documentation.

### 3. The embedded summaries become a layer

`core-doc-summaries.es.json` loads as a Spanish index whose entries hold only a summary, flagged `summary_only`. It needs no format of its own; the loader treats an entry with an empty `body_md` and the flag as a summary to merge, not as a replacement. `i18n::help_summary` and `catalog::summaries::localised` go; `set_help_language` becomes `set_docs_language`, which sets the chain the Tauri commands pass (`docs_chain`). `summaries::translations`, which feeds search, gives way to the store's own multi-language index.

With no pack installed, a Spanish reader thus sees Spanish summaries above English full documentation, marked as such. That is what the reader asked for, before anything new ships.

### 4. mxpm builds any language

`mxpm doc generate-core-docs` gains `--lang <tag>`. It reads `doc/info/<dir>` (mapping `pt-BR` to `pt_BR`), adds the English tree's shared macro files to the include path (`category-macros.texi`, `dlmf-ref-macros.texi`), and writes `<stem>-doc-index.<lang>.json` with `language` set. This is what catalog-gen does today by copying a source tree; catalog-gen's `summaries` then calls mxpm with `--lang` and keeps the summaries, as before. `mxpm doc build` gains the same `--lang`, so package authors can ship translations, writing `docs/<name>.<lang>.md` beside `docs/<name>.md`.

### 5. Language packs in core-docs

core-docs builds `maxima-core-docs-<lang>` for each language whose manual documents at least half the English manual's symbols. Each pack holds only its `doc/maxima-core-docs-doc-index.<lang>.json`, slim and full, and depends on `maxima-core-docs` at the same version. The packs don't replace English `.info` files, so Maxima's `describe` is unaffected. core-docs' CI builds every pack from the Maxima release tag it already uses, and the package index gets one entry per pack. The threshold is computed by the build and printed, so the list of packs follows Maxima's translations as they grow; today it's expected to cover `es`, `pt`, `pt-BR`, `de` and `ja`, and to leave out `ru`, `zh` and `fr`.

### 6. Choosing, in the docs panel

The docs panel's header gets a menu: "Same as Aximar" and each language with any documentation, embedded or installed, shown by its own name ("Español"). The choice is a config field, `docs_language`, with `None` meaning "Same as Aximar". The chain is that language then English.

The choice sits in the docs panel rather than Settings because readers switch while reading, for example to check the English when a translation looks stale. It also keeps the Settings window's sections as the settings-window change defines them.

When the chosen language has no full documentation installed, the panel shows a strip: "Full documentation in Spanish isn't installed", the command `mxpm install maxima-core-docs-es`, and a copy button. The strip names a pack only if the package index lists one for that language.

All new text goes through Lingui with translator notes.

### 7. MCP and the LSP

`get_function_docs` and `search_functions` take an optional `language`; results gain `language` per text, and `fallback_from` and `stale` where they apply. Without `language` the chain is English alone, so existing clients get what they get today. maxima-lsp stores `InitializeParams.locale` and builds its chain from it; VS Code sends its display language there.

### What changes for MCP clients and saved notebooks

MCP clients gain optional fields and see no change unless they pass `language`. Notebooks don't store documentation.

## Risks / Trade-offs

- **[Translations lag the English manual]** The Spanish manual dates from 2018. → The stale note, from comparing signature sets, catches the most harmful drift (missing or changed arguments). A signature set that matches can still hide a changed meaning; the note can't catch that, and the docs say so.
- **[Memory]** Holding several full languages roughly multiplies the catalog's size by the number installed. → Only installed packs load, a full pack is a few MB, and most readers install one.
- **[Packs drift from the English version]** → Each pack depends on `maxima-core-docs` at its own version, and CI builds them together from one Maxima tag.
- **[Name and field disagree in a third-party file]** → Skipped with a warning naming the file, rather than guessing.
- **[Five repositories]** → The Aximar side works on its own with the embedded Spanish summaries, so it can land first; mxpm, core-docs and the package index follow, each in its own PR.
