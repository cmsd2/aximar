# maxima-lsp

Language Server Protocol (LSP) server for Maxima `.mac` files. Provides IDE features — completions, hover, diagnostics, go-to-definition, and more — for any editor that speaks LSP.

No running Maxima process is required. All features work offline using Aximar's built-in function catalog (2500+ functions), documentation database, and a fault-tolerant `.mac` file parser.

## Features

| Feature | Description |
|---------|-------------|
| **Completions** | Built-in functions, package functions, and user-defined symbols. Includes signatures and descriptions. |
| **Hover** | Documentation for built-in functions (signatures, descriptions, examples, "see also"). Doc comments on user-defined functions. |
| **Signature help** | Parameter hints as you type inside function calls. Highlights the active parameter. |
| **Diagnostics** | Parse errors from `.mac` files shown as squiggles and in the Problems panel. |
| **Go-to-definition** | Jump to where a user-defined function or variable is defined, across open files. |
| **Find references** | Locate all uses of a symbol across open files. |
| **Document symbols** | Outline of functions, macros, and variables in the current file. |
| **Workspace symbols** | Search all symbols across all open files (`Ctrl+T`). |
| **Folding ranges** | Collapse multi-line function definitions, variable assignments, and block comments. |

## Building

From the workspace root:

```sh
cargo build --release -p maxima-lsp
```

The binary is at `target/release/maxima-lsp`.

To install it on your PATH:

```sh
cargo install --path crates/maxima-lsp
```

## Running

`maxima-lsp` communicates via JSON-RPC over **stdin/stdout** (the standard LSP transport). You don't run it directly — your editor starts it as a subprocess.

Logging goes to **stderr** and is controlled by the `RUST_LOG` environment variable:

```sh
# Default level is info
RUST_LOG=debug maxima-lsp
```

To verify the binary works, you can start it manually and send an `initialize` request, but in practice you'll configure your editor to launch it automatically (see below).

## Editor Setup

### VS Code

> A dedicated VS Code extension with full integration is planned. In the meantime, you can use a generic LSP client extension.

Install the [maxima-extension](https://github.com/yshl/maxima-extension) for syntax highlighting, then add LSP support with one of these approaches:

**Option A: Using [vscode-lsp-client](https://marketplace.visualstudio.com/items?itemName=nicolo-ribaudo.lsp-client)**

Add to your VS Code `settings.json`:

```json
{
  "lsp-client.serverCommands": {
    "maxima": {
      "command": "maxima-lsp",
      "languages": ["maxima"]
    }
  }
}
```

**Option B: Using a `tasks.json` + generic LSP extension**

Any extension that lets you configure a custom language server (e.g., [glsp](https://marketplace.visualstudio.com/items?itemName=AZMCode.generic-lsp), or the built-in LSP support in VS Code Insiders) can launch `maxima-lsp` as the server command for the `maxima` language ID.

### Neovim (nvim-lspconfig)

Add to your Neovim configuration (Lua):

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Register maxima-lsp as a custom server
if not configs.maxima_lsp then
  configs.maxima_lsp = {
    default_config = {
      cmd = { 'maxima-lsp' },
      filetypes = { 'maxima' },
      root_dir = lspconfig.util.find_git_ancestor,
      settings = {},
    },
  }
end

lspconfig.maxima_lsp.setup({})
```

You'll also need filetype detection. Add to `~/.config/nvim/ftdetect/maxima.lua`:

```lua
vim.filetype.add({
  extension = {
    mac = 'maxima',
    max = 'maxima',
    wxm = 'maxima',
  },
})
```

### Emacs (lsp-mode)

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(maxima-mode . "maxima"))
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("maxima-lsp"))
    :activation-fn (lsp-activate-on "maxima")
    :server-id 'maxima-lsp)))
```

### Emacs (eglot)

```elisp
(add-to-list 'eglot-server-programs '(maxima-mode "maxima-lsp"))
```

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "maxima"
scope = "source.maxima"
file-types = ["mac", "max", "wxm"]
language-servers = ["maxima-lsp"]

[language-server.maxima-lsp]
command = "maxima-lsp"
```

### Sublime Text (LSP package)

Add to LSP settings (`Preferences > Package Settings > LSP > Settings`):

```json
{
  "clients": {
    "maxima-lsp": {
      "enabled": true,
      "command": ["maxima-lsp"],
      "selector": "source.maxima"
    }
  }
}
```

### Other editors

Any editor with LSP support can use `maxima-lsp`. The server expects:

- **Transport:** stdio (JSON-RPC over stdin/stdout)
- **Language ID:** `maxima`
- **File extensions:** `.mac`, `.max`, `.wxm`
- **Command:** `maxima-lsp` (must be on PATH, or use the absolute path to the binary)

## Testing

### Running the test suite

```sh
# Run maxima-lsp tests only
cargo test -p maxima-lsp

# Run tests for the parser as well
cargo test -p maxima-mac-parser

# Run all workspace tests
cargo test --workspace
```

### What's tested

Unit tests cover the core helper functions and conversion logic:

- **Position and offset conversion** — byte offsets to LSP line:character positions, including multi-byte UTF-8 and UTF-16 surrogate pairs
- **Word extraction** — identifying the Maxima identifier at a cursor position (including `%`-prefixed constants)
- **Enclosing call detection** — finding which function call the cursor is inside and which parameter is active, with proper handling of nested calls, strings, and comments
- **Signature parameter extraction** — parsing function signatures into parameter lists
- **Diagnostic conversion** — mapping parser errors to LSP diagnostic objects with correct severity and ranges

### Manual testing

You can test the server interactively by piping LSP messages to it. Each message needs a `Content-Length` header:

```sh
echo 'Content-Length: 149\r\n\r\n{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"capabilities":{},"processId":null,"rootUri":null}}' | maxima-lsp 2>/dev/null
```

For a more ergonomic experience, use a tool like [lsp-devtools](https://github.com/swyddfa/lsp-devtools) to inspect the message traffic between your editor and the server.

## Developing

### Architecture

```
maxima-lsp
├── aximar-core         # Function catalog, docs, package catalog
└── maxima-mac-parser   # .mac file lexer and parser (chumsky-based)
```

The server is built on [tower-lsp](https://github.com/ebkalderon/tower-lsp), an async LSP framework for Rust. The main struct is `MaximaLsp` in `src/server.rs`, which implements the `LanguageServer` trait.

### Source layout

```
crates/maxima-lsp/src/
├── main.rs        # Binary entry point — stdio transport setup
├── lib.rs         # Public module exports
├── server.rs      # MaximaLsp struct, LanguageServer trait impl, capability declarations
├── completion.rs  # textDocument/completion — catalog + package + user symbol completions
├── hover.rs       # textDocument/hover — docs lookup with fallback chain
├── signature.rs   # textDocument/signatureHelp — parameter hints
├── definition.rs  # textDocument/definition and textDocument/references
├── symbols.rs     # textDocument/documentSymbol — AST to LSP symbol conversion
├── folding.rs     # textDocument/foldingRange — functions, assignments, block comments
├── document.rs    # DocumentState — parsed file content + diagnostics
├── helpers.rs     # Cursor utilities: word extraction, offset conversion, call detection
└── convert.rs     # Parser span/error → LSP range/diagnostic conversion
```

### How it works

1. **On file open/change:** The server receives the full file text, parses it with `maxima-mac-parser`, stores the result in a `DashMap<Url, DocumentState>`, and publishes diagnostics from any parse errors.

2. **On completion/hover/signature requests:** The server extracts the word or function call at the cursor position using helpers, then looks it up across multiple sources — the built-in catalog (2500+ functions), documentation database, package catalog, and user-defined symbols from all open files.

3. **On definition/references:** The server searches the parsed items (`MacItem` — function definitions, macro definitions, variable assignments) across all open documents. Definition searches the current file first; references searches all files with whole-word matching.

4. **On workspace symbol:** All parsed items across all open documents are searched by case-insensitive substring matching.

### Key types

- `MaximaLsp` (`server.rs`) — The server instance. Holds the LSP client handle, catalogs, and the document map.
- `DocumentState` (`document.rs`) — Stores a file's content, version, and parsed `MacFile`.
- `MacFile` (`maxima-mac-parser`) — The parse result: a list of `MacItem`s (functions, macros, variables), `LoadCall`s, and `ParseError`s.
- `Catalog`, `Docs`, `PackageCatalog` (`aximar-core`) — Built-in function metadata, loaded once at startup.

### Adding a new LSP feature

1. **Declare the capability** in `server.rs` inside the `initialize` method's `ServerCapabilities` struct.
2. **Implement the handler** — add the corresponding `LanguageServer` trait method in `server.rs`, delegating to a new module if the logic is non-trivial.
3. **Add tests** for any helper functions or conversion logic.
4. **Update this document** with a description of the new feature.

### Running during development

```sh
# Build and run tests on changes
cargo watch -x 'test -p maxima-lsp'

# Build the binary for editor testing
cargo build -p maxima-lsp

# Point your editor at the debug binary
# e.g., in Neovim: cmd = { 'target/debug/maxima-lsp' }
```

To see detailed server logs, set `RUST_LOG=debug` or `RUST_LOG=maxima_lsp=trace` in your editor's environment.

### Related crates

| Crate | Role |
|-------|------|
| `maxima-mac-parser` | Lexer and fault-tolerant parser for `.mac` files. Produces `MacFile` with items, load calls, and errors. |
| `aximar-core` | Shared library with `Catalog` (function search), `Docs` (full documentation), and `PackageCatalog` (package function search). Also contains the Maxima session manager and notebook types. |
| `aximar-mcp` | MCP server for the Aximar notebook app. Shares `aximar-core` but is otherwise independent of the LSP. |

### Parser details

`maxima-mac-parser` uses [chumsky](https://github.com/zesterer/chumsky) for lexing and a custom fault-tolerant structural parser. It extracts:

- **Function definitions:** `f(x, y) := body` — name, parameters, spans, attached doc comments
- **Macro definitions:** `f(x) ::= body`
- **Variable assignments:** `name : value`
- **Load calls:** `load("path")` — for future dependency tracking

The parser recovers from errors by skipping to the next `;` or `$` terminator, so partial files still produce useful results for IDE features. See `docs/maxima-mac-syntax.md` for syntax details and common Maxima pitfalls.
