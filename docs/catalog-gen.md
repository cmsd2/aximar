# Catalog Generator Tool

`catalog-gen` is a standalone Rust tool that generates `catalog.json` from Maxima's official Texinfo documentation.

## Overview

The function catalog (`src-tauri/src/catalog/catalog.json`) is embedded at compile time and provides autocomplete, hover tooltips, and search. The catalog-gen tool parses Maxima's comprehensive Texinfo docs to extract function definitions, signatures, descriptions, examples, and categories — producing a much larger catalog than hand-writing.

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
  --merge                  Merge with existing catalog at output path (hand-written entries take priority)
  -q, --quiet              Suppress informational output (e.g. unmapped category warnings)
  --min-description <N>    Skip entries with descriptions shorter than N chars [default: 10]
```

### `catalog-gen from-xml`

Parse a pre-existing Maxima XML file.

```
Arguments:
  <INPUT>                  Path to the Maxima XML file

Options:
  -o, --output <PATH>      Output path for catalog.json [default: src-tauri/src/catalog/catalog.json]
  --merge                  Merge with existing catalog at output path (hand-written entries take priority)
  -q, --quiet              Suppress informational output (e.g. unmapped category warnings)
  --min-description <N>    Skip entries with descriptions shorter than N chars [default: 10]
```

## Merge Behavior

When `--merge` is provided, the tool reads the existing catalog at the output path before writing. Entries from the existing catalog override generated ones by name. This preserves hand-tuned descriptions for important functions while filling in the rest automatically from the docs.

## Category Mapping

Maxima uses fine-grained categories (e.g., "Differential calculus", "Integral calculus"). The tool maps these to the `FunctionCategory` enum used by the app. Unmapped categories are logged to stderr by default (suppress with `--quiet`), then update `mapping.rs` as needed.

## Architecture

```
Maxima source (doc/info/*.texi)
        |
        v  makeinfo --xml        (automated by `generate`)
   maxima.xml (~10-20MB)
        |
        v  XML parsing           (automated by `generate` / `from-xml`)
   catalog.json (500+ functions)
        |
        v  include_str!()        (at compile time)
   Embedded in aximar binary
```

The generated `catalog.json` is checked into the repo. The tool is run manually when updating to a new Maxima version — it is NOT part of the regular build.
