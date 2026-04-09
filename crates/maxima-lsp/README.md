# maxima-lsp

Language Server Protocol (LSP) server for Maxima `.mac` files. Provides IDE features in VS Code and any LSP-capable editor — no running Maxima process required.

## Features

- **Completions** — 2500+ built-in functions, package functions, and user-defined symbols
- **Hover documentation** — Signatures, descriptions, examples, and "see also" links
- **Signature help** — Parameter hints as you type inside function calls
- **Go-to-definition** — Jump to where a function or variable is defined
- **Find references** — Locate all uses of a symbol across open files
- **Document symbols** — Outline of functions, macros, and variables
- **Workspace symbols** — Search all symbols across open files
- **Diagnostics** — Parse errors shown as squiggles in the editor
- **Folding** — Collapse multi-line definitions and block comments

## Building

```sh
cargo build --release -p maxima-lsp
```

To install on your PATH:

```sh
cargo install --path crates/maxima-lsp
```

## Usage

`maxima-lsp` communicates via JSON-RPC over stdin/stdout using the LSP protocol. Your editor starts it as a subprocess — you don't run it directly.

With the [Maxima VS Code extension](https://github.com/cmsd2/maxima-extension), the server is found automatically if it's on your PATH. You can also set `maxima.lsp.path` in VS Code settings to point to the binary.

Logging goes to stderr and is controlled by `RUST_LOG`:

```sh
RUST_LOG=maxima_lsp=debug maxima-lsp
```

## Architecture

The server is built on [tower-lsp](https://github.com/ebkalderon/tower-lsp) and uses `maxima-mac-parser` for fault-tolerant parsing and `aximar-core` for the function catalog.

```
crates/maxima-lsp/src/
├── main.rs        # Binary entry point
├── lib.rs         # Module exports
├── server.rs      # LSP request handlers and document management
├── completion.rs  # Completion provider
├── definition.rs  # Go-to-definition and find references
├── document.rs    # Document state (parsed MacFile per URI)
├── folding.rs     # Folding range provider
├── helpers.rs     # Symbol extraction and word-at-position utilities
├── hover.rs       # Hover documentation provider
├── convert.rs     # Span conversion between parser and LSP types
├── signature.rs   # Signature help provider
└── symbols.rs     # Document and workspace symbol providers
```

See the full documentation at [`docs/maxima-lsp.md`](../../docs/maxima-lsp.md).
