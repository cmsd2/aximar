# Catalog Generator Tool

`catalog-gen` is a standalone Rust tool that generates `catalog.json`, `docs.json`, and `packages.json` from Maxima's official Texinfo documentation and installed share packages.

## Overview

The function catalog (`crates/aximar-core/src/catalog/catalog.json`) is embedded at compile time and provides autocomplete, hover tooltips, and search. The full documentation (`crates/aximar-core/src/catalog/docs.json`) is also embedded and powers the Docs panel with rich Markdown content including code examples, math, cross-references, and figures. The package catalog (`crates/aximar-core/src/catalog/packages.json`) provides loadable package discovery, `load()` autocomplete, and "did you mean to load X?" error suggestions. The catalog-gen tool parses Maxima's comprehensive Texinfo docs to extract function definitions, signatures, descriptions, examples, and categories — producing a much larger catalog than hand-writing.

## Prerequisites

- `git` (for cloning Maxima source, unless using `--maxima-src`)
- [makeinfo](https://www.gnu.org/software/texinfo/) (part of GNU Texinfo)

## Usage

### Full pipeline (recommended)

One command handles everything — cloning Maxima source, converting Texinfo to XML, parsing, and writing the catalog:

```bash
cargo run -p catalog-gen -- generate
```

With merge (preserves hand-written entries):

```bash
cargo run -p catalog-gen -- generate --merge
```

If you already have a Maxima source checkout:

```bash
cargo run -p catalog-gen -- generate --maxima-src /path/to/maxima-code
```

### From pre-existing XML

If you already have a `maxima.xml` file (produced by `makeinfo --xml`):

```bash
cargo run -p catalog-gen -- from-xml maxima.xml
```

## CLI Reference

### `catalog-gen generate`

Full pipeline: clone Maxima source (or use local), run makeinfo, parse XML, write catalog.

When no `--maxima-src` is given, the tool caches the Maxima clone at `/tmp/maxima-src`. On subsequent runs it fetches and resets to the latest HEAD instead of re-cloning.

```
Options:
  --maxima-src <PATH>      Path to existing Maxima source checkout (skips clone/fetch)
  --git-ref <REF>          Git ref to checkout when cloning/fetching [default: master]
  -o, --output <PATH>      Output path for catalog.json [default: src-tauri/src/catalog/catalog.json]
  --docs-output <PATH>     Output path for docs.json [default: src-tauri/src/catalog/docs.json]
  --merge                  Merge with existing catalog at output path (hand-written entries take priority)
  -q, --quiet              Suppress informational output (e.g. unmapped category warnings)
  --min-description <N>    Skip entries with descriptions shorter than N chars [default: 10]
```

The `generate` command also copies PNG figures from the Maxima source (`doc/info/figures/`) into `src-tauri/src/catalog/figures/` for use in the documentation browser.

### `catalog-gen packages`

Scan the Maxima share directory to discover loadable packages and the functions they provide. Produces `packages.json` with function names, descriptions, and signatures.

Like `generate`, when no `--maxima-src` is given, the tool caches the Maxima clone at `/tmp/maxima-src`. On subsequent runs it fetches and resets to the latest HEAD instead of re-cloning.

```bash
# Default: clone/update Maxima source at /tmp/maxima-src
cargo run -p catalog-gen -- packages

# From a local Maxima source checkout:
cargo run -p catalog-gen -- packages --maxima-src /path/to/maxima-code

# From an installed Maxima share directory (no git):
cargo run -p catalog-gen -- packages --share-dir /opt/homebrew/share/maxima/5.48.1/share

# With catalog.json for Texinfo-derived signatures:
cargo run -p catalog-gen -- packages --catalog crates/aximar-core/src/catalog/catalog.json
```

```
Options:
  --maxima-src <PATH>      Path to existing Maxima source checkout (skips clone/fetch; conflicts with --share-dir)
  --share-dir <PATH>       Path to an installed Maxima share directory (conflicts with --maxima-src)
  --git-ref <REF>          Git ref to checkout when cloning/fetching [default: master]
  --catalog <PATH>         Path to a catalog.json to cross-reference function signatures (Texinfo signatures override regex)
  -o, --output <PATH>      Output path for packages.json [default: crates/aximar-core/src/catalog/packages.json]
```

The scanner discovers packages by examining subdirectories of the share directory. It extracts function definitions and signatures from `.mac` files using regex, and generates descriptions from well-known package metadata, file header comments, or the function catalog. When `--catalog` is provided (or when run as part of `generate`), Texinfo-derived signatures from the catalog take priority over regex-extracted ones.

Note: The `generate` command automatically produces `packages.json` alongside `catalog.json` and `docs.json` when a share directory is available (always the case when using `--maxima-src` or the default clone).

### `catalog-gen from-xml`

Parse a pre-existing Maxima XML file.

```
Arguments:
  <INPUT>                  Path to the Maxima XML file

Options:
  -o, --output <PATH>      Output path for catalog.json [default: src-tauri/src/catalog/catalog.json]
  --docs-output <PATH>     Output path for docs.json [default: src-tauri/src/catalog/docs.json]
  --merge                  Merge with existing catalog at output path (hand-written entries take priority)
  -q, --quiet              Suppress informational output (e.g. unmapped category warnings)
  --min-description <N>    Skip entries with descriptions shorter than N chars [default: 10]
```

## Merge Behavior

When `--merge` is provided, the tool reads the existing catalog at the output path before writing. Entries from the existing catalog override generated ones by name. This preserves hand-tuned descriptions for important functions while filling in the rest automatically from the docs.

## Category Mapping

Maxima uses fine-grained categories (e.g., "Differential calculus", "Integral calculus"). The tool maps these to the `FunctionCategory` enum used by the app. Unmapped categories are logged to stderr by default (suppress with `--quiet`), then update `mapping.rs` as needed.

## Outputs

The tool produces three JSON files and optionally copies figure images:

- **`catalog.json`** — Lean function catalog with short descriptions, signatures, categories, and examples. Used for autocomplete, hover tooltips, and search.
- **`docs.json`** — Full Markdown documentation per function (2000+ entries). Maps function names to Markdown strings with code blocks, math, cross-references, lists, tables, and figure images. Used by the Docs panel.
- **`packages.json`** — Loadable package catalog (100+ packages). Lists each package's load path, description, exported functions, and function signatures. Used for `load()` autocomplete, command palette package browsing, docs panel signature display, and "did you mean to load X?" error suggestions.
- **`figures/`** — PNG figures from Maxima's documentation, copied during `generate`.

### docs.json format

```json
{
  "diff": "Returns the derivative or differential of *expr*...\n\n```maxima\n(%i1) diff(x^3, x);\n```\n\nSee also: [depends](fn:depends), [del](fn:del)\n",
  "integrate": "..."
}
```

### Markdown features in docs.json

| Feature | Syntax |
|---|---|
| Code blocks | ` ```maxima ``` ` |
| Inline math | `$...$` |
| Display math | `$$...$$` |
| Cross-references | `[name](fn:name)` |
| Figures | `![desc](figures/name.png)` |
| Bold/italic/code | Standard Markdown |
| Lists | `- ` and `1. ` |
| Tables | Pipe tables |

### packages.json format

```json
[
  {
    "name": "distrib",
    "description": "Probability distributions (normal, t, chi-squared, F, beta, etc.)",
    "functions": ["cdf_normal", "pdf_normal", "..."],
    "signatures": {
      "cdf_normal": "cdf_normal(x, m, s)",
      "pdf_normal": "pdf_normal(x, m, s)"
    }
  }
]
```

The `signatures` field maps function names to their call signatures. Signatures are extracted from two sources: regex on `.mac` file definitions (fallback) and Texinfo-parsed catalog entries (takes priority when `--catalog` is provided or when run via `generate`). The field is omitted when empty.

## Architecture

```
Maxima source (doc/info/*.texi)
        |
        v  makeinfo --xml        (automated by `generate`)
   maxima.xml (~10-20MB)
        |
        v  XML parsing           (automated by `generate` / `from-xml`)
   ├── catalog.json (2500+ functions, lean)
   └── docs.json    (2500+ entries, full Markdown)
        |
        v  include_str!()        (at compile time)
   Embedded in aximar binary
```

The generated files are checked into the repo. The tool is run manually when updating to a new Maxima version — it is NOT part of the regular build.
