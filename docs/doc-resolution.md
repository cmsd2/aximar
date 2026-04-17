# Documentation Resolution Order

How documentation lookups and searches resolve across data sources in Aximar.

## Data Sources

`Catalog::load()` ingests these in order. Later sources overwrite earlier ones by symbol name (case-insensitive).

| Priority | Source | Location | Content |
|----------|--------|----------|---------|
| 1 (lowest) | ax-plotting | `ax_plotting_catalog.json` + `ax_plotting_docs.json` (embedded) | 10 functions, full body_md |
| 2 | Core slim index | `core-doc-index.json` (embedded, ~2700 symbols) | signature, summary, category, section, keywords. No body_md, examples, or see_also |
| 3 (highest) | Runtime packages | `~/.maxima/*/doc/*-doc-index.json` | Full doc-index from installed packages; overwrites earlier entries |

Separately, `PackageCatalog` loads `packages.json` (embedded) for function-to-package mapping, package search, and package function completions.

After ingestion, a BM25 search index is built over three fields:

| Field | Weight |
|-------|--------|
| name | 3.0 |
| summary | 1.0 |
| keywords | 2.0 |

## DocIndexStore Methods

| Method | What it does |
|--------|-------------|
| `get(name)` | Case-insensitive symbol lookup. Returns `(package, SymbolEntry)` |
| `search(query)` | BM25 search, max 20 results. Returns name, signature, summary, package, score |
| `complete(prefix)` | Case-insensitive prefix match on symbol names |
| `hover_markdown(name)` | Formatted markdown: signature + body_md (or summary) + examples + see_also |
| `signature_info(name)` | Primary + alternative signatures with extracted parameter names |
| `find_similar(name, dist)` | Levenshtein distance search, max 5 results |
| `find_deprecated()` | Symbols with "deprecated" or "obsolete" in summary |
| `by_category()` | Group symbols by category field |

## LSP

| Endpoint | Resolution order |
|----------|-----------------|
| **Hover** | 1. `catalog.hover_markdown()` 2. Open document symbols (user-defined functions/variables) 3. `packages` (function-to-package mapping) |
| **Completion** | 1. `catalog.complete()` 2. `packages.complete_functions()` (deduped) 3. Document symbols (deduped) |
| **Signature Help** | 1. `catalog.signature_info()` 2. Document symbols 3. `packages` |
| **searchFunctions** (execute command) | 1. `catalog.search()` (BM25, max 20) 2. `packages.search_functions()` (deduped) |
| **getFunctionDocs** (execute command) | 1. `catalog.get()` — returns body_md if available, else None. All signatures (primary + alternatives), summary, category, examples, see_also |

The LSP also watches `~/.maxima/*/doc/*-doc-index.json` for changes and reloads the catalog automatically when packages are installed or removed.

## MCP

| Tool | Resolution order |
|------|-----------------|
| **search_functions** | `catalog.search()` only (BM25, max 20) |
| **get_function_docs** | 1. `catalog.get()` — uses body_md if available, falls back to summary 2. If not found, `catalog.find_similar()` for "did you mean" suggestions |
| **complete_function** | 1. `catalog.complete()` 2. `packages.complete_functions()` (deduped) |
| **list_deprecated** | `catalog.find_deprecated()` |

## Tauri (GUI)

| Command | Resolution order |
|---------|-----------------|
| **search_functions** | `catalog.search()` then `catalog.get()` per result, converted to `MaximaFunction` |
| **complete_function** | 1. `catalog.complete()` 2. `packages.complete_functions()` (deduped) |
| **get_function** | `catalog.get()` converted to `MaximaFunction` |
| **get_function_docs** | `catalog.get()` — returns body_md if non-empty, else None |
| **list_categories** | `catalog.by_category()` then `catalog.get()` per symbol |

## Error Enhancement

When Maxima returns an "undefined function" error during evaluation:

1. `packages.package_for_function(name)` — if found, suggests `load("package")$`
2. `catalog.find_similar(name, 3)` — if found, suggests similar names ("did you mean?")

## Gaps and Notes

- **MCP `search_functions`** does not search packages (unlike LSP and Tauri completion which include package functions).
- **Core functions have no `body_md`** from the slim embedded index. Full documentation becomes available when the user installs `maxima-core-docs` via `mxpm install maxima-core-docs`. The slim index still provides signatures, summaries, categories, and keywords for search/completion/hover.
- **Tauri `get_function_docs`** returns `None` for core functions without installed full docs (no summary fallback — the GUI should use `get_function` for the summary instead).
- **ax-plotting functions** have full body_md embedded because they ship with Aximar and aren't installable separately yet.
- **Package metadata** (`packages.json`) only has function names and signatures, not full documentation. It's used to suggest `load()` calls and to supplement completion results.
