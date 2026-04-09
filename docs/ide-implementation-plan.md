# IDE Integration: Implementation Plan

Detailed build plan for VS Code integration with Maxima via LSP and DAP, building on [maxima-extension](https://github.com/yshl/maxima-extension) and the aximar Rust crates.

Companion docs: [ide-features.md](ide-features.md) (architecture), [vscode-integration-usecases.md](vscode-integration-usecases.md) (use cases), [maxima-debugger-internals.md](maxima-debugger-internals.md) (debugger reference).

---

## Workspace Layout (Target State)

```
aximar/
  Cargo.toml                          # Add maxima-lsp, maxima-dap members
  crates/
    aximar-core/                      # Existing — shared library
    aximar-mcp/                       # Existing — MCP server
    maxima-lsp/                       # NEW — Language Server Protocol
    maxima-dap/                       # NEW — Debug Adapter Protocol
    maxima-mac-parser/                # NEW — .mac file parser (shared by LSP + DAP)
  maxima-extension/                   # Forked VS Code extension (submodule or subtree)
```

---

## Phase 1: Extension Scaffold + Grammar Refinement

**Goal:** A published VS Code extension with improved syntax highlighting and a "Run File in Terminal" command. No Rust binaries yet.

### 1.1 Fork and restructure maxima-extension

Fork `yshl/maxima-extension` into the aximar org. Add it as a subdirectory of the workspace (or a git submodule).

Add TypeScript tooling:

```
maxima-extension/
  package.json                  # Enhanced (see below)
  tsconfig.json                 # NEW
  src/
    extension.ts                # NEW — activation, command registration
  syntaxes/
    maxima.tmLanguage.json      # Enhanced grammar
  language-configuration.json   # Enhanced (add indentation rules)
  .vscodeignore
```

**Dependencies to add:**
- `vscode-languageclient` (for Phase 2, but declare the npm dep now)
- `@types/vscode`
- `esbuild` or `tsup` for bundling

### 1.2 Enhance the TextMate grammar

The existing grammar at `syntaxes/maxima.tmLanguage.json` (103 lines) handles keywords, strings, comments (nested), constants, and generic function calls. Add:

**Definition operators** — `:=` and `::=` should be `keyword.operator.definition.maxima`, distinct from `:` (assignment) and `::` (eval-assign) which are `keyword.operator.assignment.maxima`.

```json
{
  "name": "keyword.operator.definition.maxima",
  "match": "::?="
}
```

**Missing keywords** — `block`, `lambda`, `catch`, `throw`, `error`, `errcatch` as `keyword.control.maxima`.

**Statement terminators** — `;` as `punctuation.terminator.display.maxima`, `$` as `punctuation.terminator.suppress.maxima`. Enables distinct theming.

**`:lisp` escape** — Match `:lisp` at line start, switch remainder to a Lisp embedded scope:

```json
{
  "begin": "^\\s*:lisp\\b",
  "beginCaptures": { "0": { "name": "keyword.control.lisp-escape.maxima" } },
  "end": "$",
  "name": "meta.embedded.lisp.maxima",
  "patterns": [{ "include": "source.commonlisp" }]
}
```

**Function definition site** — `f(x) :=` should highlight `f` as `entity.name.function.maxima`:

```json
{
  "match": "\\b([a-zA-Z_][a-zA-Z0-9_]*)\\s*\\([^)]*\\)\\s*::?=",
  "captures": {
    "1": { "name": "entity.name.function.definition.maxima" }
  }
}
```

**Underscored identifiers** — Maxima allows `_` in names. The existing `[[:alnum:]]` pattern should be `[a-zA-Z_][a-zA-Z0-9_]*` for correctness.

### 1.3 Enhance language-configuration.json

Add indentation rules for `block(`, `if ... then`, `for ... do`:

```json
{
  "indentationRules": {
    "increaseIndentPattern": "\\b(block|if|for|while|do|then|else)\\b.*(?:,|\\()\\s*$",
    "decreaseIndentPattern": "^\\s*\\)\\s*[;$]?\\s*$"
  },
  "wordPattern": "[a-zA-Z_%][a-zA-Z0-9_%]*"
}
```

Add `onEnterRules` for comment continuation (`/* ... */`).

### 1.4 Add "Run File" command

In `src/extension.ts`:

```typescript
import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
  context.subscriptions.push(
    vscode.commands.registerCommand('maxima.runFile', () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor) return;
      const file = editor.document.uri.fsPath;
      const terminal = vscode.window.createTerminal('Maxima');
      terminal.show();
      terminal.sendText(`maxima --very-quiet --batch "${file}"`);
    })
  );
}
```

Register in `package.json`:

```json
{
  "activationEvents": ["onLanguage:maxima"],
  "main": "./out/extension.js",
  "contributes": {
    "commands": [
      { "command": "maxima.runFile", "title": "Maxima: Run File" }
    ],
    "menus": {
      "editor/context": [
        { "command": "maxima.runFile", "when": "editorLangId == maxima" }
      ]
    }
  }
}
```

### 1.5 Deliverables

- [ ] Forked repo with TypeScript build setup
- [ ] Enhanced grammar (definition ops, block/lambda, terminators, :lisp, function def sites)
- [ ] Indentation rules and word pattern
- [ ] "Run File" command via terminal
- [ ] Extension published to VS Code Marketplace

---

## Phase 2: maxima-mac-parser Crate

**Goal:** A shared `.mac` file parser that extracts symbols, definitions, and structure. Used by both LSP (for document symbols, go-to-definition) and DAP (for breakpoint mapping).

### 2.1 Crate setup

```toml
# crates/maxima-mac-parser/Cargo.toml
[package]
name = "maxima-mac-parser"
version = "0.1.0"
edition = "2021"

[dependencies]
# Minimal — no async, no Maxima process needed
```

This crate has **no dependency on aximar-core** — it's a pure parser. aximar-core, maxima-lsp, and maxima-dap all depend on it.

### 2.2 Parser output types

```rust
/// A parsed .mac file
pub struct MacFile {
    pub path: PathBuf,
    pub items: Vec<MacItem>,
    pub load_calls: Vec<LoadCall>,
    pub errors: Vec<ParseError>,
}

/// Top-level item in a .mac file
pub enum MacItem {
    FunctionDef(FunctionDef),
    VariableAssign(VariableAssign),
    MacroDef(MacroDef),        // f(x) ::= ...
    Comment(Comment),
}

pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub span: Span,             // byte range in source
    pub name_span: Span,        // just the name, for go-to-definition highlighting
    pub body_span: Span,        // for breakpoint offset calculation
    pub doc_comment: Option<String>,  // /* ... */ immediately before def
}

pub struct VariableAssign {
    pub name: String,
    pub span: Span,
    pub name_span: Span,
}

pub struct LoadCall {
    pub path: String,           // argument to load()
    pub span: Span,
}

pub struct Span {
    pub start: Position,
    pub end: Position,
    pub byte_start: usize,
    pub byte_end: usize,
}

pub struct Position {
    pub line: u32,              // 0-based
    pub character: u32,         // 0-based, UTF-16 code units (LSP convention)
}
```

### 2.3 Parser strategy

This is **not** a full Maxima parser. It's a lightweight, fault-tolerant extractor that handles the common patterns. It runs on every keystroke (via LSP incremental sync), so it must be fast.

**Approach:** Line-oriented scan with minimal state tracking.

1. **Tokenize** — Split into tokens: identifiers, operators (`:=`, `::=`, `:`, `(`, `)`, etc.), strings, comments, terminators.
2. **Extract definitions** — Match patterns:
   - `IDENT ( PARAMS ) := ...` → FunctionDef
   - `IDENT ( PARAMS ) ::= ...` → MacroDef
   - `IDENT : ...` → VariableAssign (only at top level or in block locals)
   - `load ( STRING )` → LoadCall
3. **Track nesting** — Count `(` and `)` to find function body boundaries (needed for breakpoint offset computation). Handle `block([locals], ...)`.
4. **Recover from errors** — If parsing fails, skip to the next `;` or `$` and continue. The parser should never panic or refuse to produce partial results.

**Comments as doc comments:** A `/* ... */` comment immediately before a function definition (no blank line between) is treated as a doc comment and attached to the `FunctionDef`.

### 2.4 Testing

- Unit tests with inline `.mac` snippets
- Test edge cases: nested `block()`, multi-line function defs, `define(f(x), ...)` form, comments between definition parts
- Benchmark: parse a 1000-line `.mac` file in under 5ms

### 2.5 Deliverables

- [ ] `maxima-mac-parser` crate with `parse(source: &str) -> MacFile`
- [ ] Position tracking (line, character, byte offset)
- [ ] Function definition extraction with parameter names
- [ ] Variable assignment extraction
- [ ] `load()` call extraction
- [ ] Doc comment attachment
- [ ] Fault-tolerant: always produces partial results
- [ ] Unit tests, including edge cases from `docs/maxima-mac-syntax.md`

---

## Phase 3: maxima-lsp Crate (Offline Features)

**Goal:** An LSP server providing completions, hover, signature help, document symbols, and go-to-definition — all without a running Maxima process.

### 3.1 Crate setup

```toml
# crates/maxima-lsp/Cargo.toml
[package]
name = "maxima-lsp"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "maxima-lsp"
path = "src/main.rs"

[dependencies]
aximar-core = { path = "../aximar-core" }
maxima-mac-parser = { path = "../maxima-mac-parser" }
tower-lsp = "0.20"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
dashmap = "6"
```

**Why tower-lsp:** Async, tower-based, well-maintained, used by rust-analyzer's reference implementation. Provides the JSON-RPC framing over stdio and typed request/response handlers.

### 3.2 Server structure

```rust
// src/main.rs
#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| MaximaLsp::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}

// src/server.rs
struct MaximaLsp {
    client: Client,
    catalog: Arc<Catalog>,
    docs: Arc<Docs>,
    packages: Arc<PackageCatalog>,
    documents: DashMap<Url, DocumentState>,   // Open file states
    workspace_symbols: Arc<RwLock<WorkspaceIndex>>,
}

struct DocumentState {
    content: String,
    version: i32,
    parsed: MacFile,       // From maxima-mac-parser
}
```

### 3.3 LSP capabilities to implement

**`initialize`** — Return capabilities:
```rust
ServerCapabilities {
    text_document_sync: Some(TextDocumentSyncCapability::Kind(
        TextDocumentSyncKind::INCREMENTAL,
    )),
    completion_provider: Some(CompletionOptions {
        trigger_characters: Some(vec!["(".into(), ",".into()]),
        ..Default::default()
    }),
    hover_provider: Some(HoverProviderCapability::Simple(true)),
    signature_help_provider: Some(SignatureHelpOptions {
        trigger_characters: Some(vec!["(".into(), ",".into()]),
        ..Default::default()
    }),
    document_symbol_provider: Some(OneOf::Left(true)),
    definition_provider: Some(OneOf::Left(true)),
    references_provider: Some(OneOf::Left(true)),
    workspace_symbol_provider: Some(OneOf::Left(true)),
    folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
    ..Default::default()
}
```

**`textDocument/didOpen` and `textDocument/didChange`** — Parse the document with `maxima-mac-parser`, update `DocumentState`. Rebuild workspace symbol index.

**`textDocument/completion`** — Two sources, merged and ranked:
1. **Catalog** — `Catalog::complete(prefix)` for built-in functions (2500+ entries). Map `CompletionResult` to LSP `CompletionItem` with:
   - `label`: function name
   - `detail`: first signature
   - `documentation`: description (markdown)
   - `insert_text`: snippet with tab stops for parameters
   - `kind`: `CompletionItemKind::FUNCTION`
2. **Document symbols** — User-defined functions and variables from `MacFile.items` across all open documents and workspace files.

For package functions, annotate with the package name and offer a code action to insert `load("package")$` if not already present.

**`textDocument/hover`** — Look up the word under cursor:
1. Check catalog: `Catalog::get(name)` → show signature + description + examples
2. Check docs: `Docs::get(name)` → show full manual entry (markdown)
3. Check document symbols: show definition location and parameter names
4. Check packages: `PackageCatalog::function_package(name)` → show package info

**`textDocument/signatureHelp`** — When cursor is inside `func(` ... `)`:
1. Parse backwards to find the function name and which argument position the cursor is at
2. Look up signatures in the catalog
3. Highlight the active parameter

**`textDocument/documentSymbol`** — Map `MacFile.items` to LSP `DocumentSymbol` tree:
- `FunctionDef` → `SymbolKind::FUNCTION`
- `VariableAssign` → `SymbolKind::VARIABLE`
- `MacroDef` → `SymbolKind::FUNCTION` (with "macro" detail)
- Nest inside `block()` scopes where applicable

**`textDocument/definition`** — For a symbol under cursor:
1. Search current file's `MacFile.items` for a matching definition
2. Search workspace files
3. For built-ins, return no result (or optionally open a virtual document with the catalog docs)

**`textDocument/references`** — Grep-level: search all workspace `.mac` files for the identifier.

**`workspace/symbol`** — Return all function and variable definitions across the workspace.

**`textDocument/foldingRange`** — From the parser: `block(...)`, `if...else`, `for...do`, `/* ... */` comment blocks.

### 3.4 Wire into the VS Code extension

Add to `maxima-extension/package.json`:

```json
{
  "contributes": {
    "configuration": {
      "title": "Maxima",
      "properties": {
        "maxima.lsp.path": {
          "type": "string",
          "default": "",
          "description": "Path to maxima-lsp binary. If empty, uses bundled binary."
        },
        "maxima.lsp.enabled": {
          "type": "boolean",
          "default": true
        }
      }
    }
  }
}
```

In `extension.ts`, start the LSP client:

```typescript
import { LanguageClient, TransportKind } from 'vscode-languageclient/node';

const serverPath = config.get<string>('maxima.lsp.path') || context.asAbsolutePath('bin/maxima-lsp');
const client = new LanguageClient(
  'maxima-lsp',
  'Maxima Language Server',
  { command: serverPath, transport: TransportKind.stdio },
  { documentSelector: [{ scheme: 'file', language: 'maxima' }] }
);
client.start();
```

### 3.5 Deliverables

- [ ] `maxima-lsp` crate with stdio binary
- [ ] Completions from catalog + document symbols
- [ ] Hover with signatures, descriptions, examples
- [ ] Signature help with active parameter
- [ ] Document symbols and workspace symbols
- [ ] Go-to-definition (local and cross-file via `load()` chains)
- [ ] Find references (grep-level)
- [ ] Folding ranges
- [ ] VS Code extension updated with LSP client
- [ ] Integration tests: start server, send LSP requests, verify responses

---

## Phase 4: maxima-lsp Live Diagnostics

**Goal:** Red squiggles on errors by running a background Maxima process.

### 4.1 Background session

Add a `DiagnosticsSession` to the LSP server that:
- Spawns a Maxima process on first file save (lazy init)
- Uses aximar-core's `SessionManager` and `MaximaProcess`
- Has its own `OutputSink` that captures diagnostic output

```rust
struct DiagnosticsSession {
    session: Arc<SessionManager>,
    backend: Backend,
    maxima_path: Option<String>,
}
```

### 4.2 On-save diagnostics

When a `.mac` file is saved:

1. Debounce (500ms since last save)
2. Collect `load()` dependencies from the parser
3. Send to the background Maxima: `batchload("file.mac")$`
4. Capture output via the protocol layer
5. Parse errors using `aximar_core::maxima::errors`
6. Map error line numbers to LSP `Diagnostic` objects
7. Publish via `client.publish_diagnostics(uri, diagnostics, version)`

**Error parsing:** Maxima's `batchload` errors typically include line numbers:

```
file.mac:15:2: incorrect syntax: = is not a prefix operator
```

The error enhancement from aximar-core (`ErrorInfo` with suggestions, did-you-mean) maps directly to LSP diagnostic `relatedInformation` and code actions.

### 4.3 Dynamic completions

With a running background session, the LSP can also query:
- `values;` → user-defined variables from the session
- `functions;` → user-defined functions from the session
- After detecting `load("pkg")` in the file, `load("pkg")$ functions;` → package symbols

Merge these with catalog completions.

### 4.4 Deliverables

- [ ] Background Maxima session (lazy-started, auto-restarted on crash)
- [ ] On-save diagnostics with debounce
- [ ] Error parsing with line numbers
- [ ] Enhanced diagnostics (suggestions, did-you-mean) from aximar-core
- [ ] Dynamic completions from running session
- [ ] Graceful handling of session failures (fall back to offline mode)

---

## Phase 5: maxima-dap Crate

**Goal:** Interactive debugging in VS Code — breakpoints, stepping, call stack, variable inspection.

### 5.1 Crate setup

```toml
# crates/maxima-dap/Cargo.toml
[package]
name = "maxima-dap"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "maxima-dap"
path = "src/main.rs"

[dependencies]
aximar-core = { path = "../aximar-core" }
maxima-mac-parser = { path = "../maxima-mac-parser" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

**No existing DAP framework crate** is mature enough to depend on. Implement the [DAP spec](https://microsoft.github.io/debug-adapter-protocol/) directly using JSON-RPC over stdio. The protocol is simpler than LSP — roughly 15 request types for a minimal implementation.

### 5.2 DAP message types

Define the DAP types manually (or use `debugserver-types` if it's up to date):

```rust
// Minimal DAP types needed
mod dap {
    pub struct Request { pub seq: i64, pub command: String, pub arguments: Value }
    pub struct Response { pub seq: i64, pub request_seq: i64, pub success: bool, pub command: String, pub body: Value }
    pub struct Event { pub seq: i64, pub event: String, pub body: Value }

    // Request argument types
    pub struct InitializeArguments { ... }
    pub struct LaunchArguments { pub program: String, pub maxima_path: Option<String>, pub backend: Option<String> }
    pub struct SetBreakpointsArguments { pub source: Source, pub breakpoints: Vec<SourceBreakpoint> }
    pub struct ContinueArguments { pub thread_id: i64 }
    pub struct NextArguments { pub thread_id: i64 }
    pub struct StepInArguments { pub thread_id: i64 }
    pub struct StackTraceArguments { pub thread_id: i64 }
    pub struct ScopesArguments { pub frame_id: i64 }
    pub struct VariablesArguments { pub variables_reference: i64 }
    pub struct EvaluateArguments { pub expression: String, pub frame_id: Option<i64>, pub context: Option<String> }
    // ...
}
```

### 5.3 Server structure

```rust
struct MaximaDap {
    seq_counter: AtomicI64,
    session: SessionManager,
    backend: Backend,
    maxima_path: Option<String>,
    parser: maxima_mac_parser,

    // Breakpoint state
    breakpoints: HashMap<PathBuf, Vec<MappedBreakpoint>>,

    // Debug state
    state: DebugState,
    stopped_frames: Vec<StackFrame>,
    variable_refs: HashMap<i64, VariableScope>,
}

enum DebugState {
    NotStarted,
    Running,
    Stopped { reason: StopReason },
    Terminated,
}

struct MappedBreakpoint {
    /// User-facing: file:line
    source_line: u32,
    /// Maxima-facing: function + offset
    function_name: String,
    function_offset: u32,
    /// DAP breakpoint ID
    id: i64,
    /// Whether Maxima accepted it
    verified: bool,
}
```

### 5.4 Protocol state machine

Extend aximar-core's process reader to detect debugger mode:

```rust
enum MaximaMode {
    Normal,
    Debugger { level: u32 },  // dbm:N>
}
```

The process reader currently reads until a sentinel. For debugging, it needs a more general approach:

1. Read a line from stdout
2. If it matches `dbm:\d+>`, switch to `Debugger` mode, emit DAP `stopped` event
3. In Debugger mode, route stdin/stdout through DAP commands instead of normal eval
4. On `:resume` / `:continue`, switch back to `Normal` mode

This is the core complexity. The current `read_until_sentinel` approach in `protocol.rs` needs to be generalized into a mode-aware output router.

### 5.5 Breakpoint mapping

When `setBreakpoints` arrives with `source.path` and line numbers:

1. Parse the file with `maxima-mac-parser`
2. For each breakpoint line, find the enclosing `FunctionDef`
3. Compute offset: `breakpoint_line - function_def.body_span.start.line`
4. Send `:break function_name offset` to Maxima
5. If the line is not inside any function, mark the breakpoint as `verified: false` with message "Breakpoints only work inside function definitions"

**Reverse mapping** (breakpoint hit → source location):
When Maxima outputs `Bkpt N:(func line)`, parse it and look up the original source file and line from `MappedBreakpoint`.

### 5.6 Stack trace parsing

When the DAP requests `stackTrace`, send `:bt` to Maxima and parse the output. Maxima's backtrace format (SBCL):

```
#0: f(x=5) (test.mac line 3)
#1: g(a=10)(test.mac line 8)
```

Parse into DAP `StackFrame` objects. Reverse-map function+line to source file:line using the `.mac` parser data.

### 5.7 Variable inspection

For `scopes` and `variables` requests:

1. Send `:frame N` to Maxima at the `dbm:` prompt
2. Parse the output for local variable bindings
3. For expandable values (lists, matrices), use lazy resolution: return a `variablesReference` ID, and on `variables` request, evaluate `part(expr, i)` for each element

For the Debug Console (`evaluate` request), send the expression directly at the `dbm:` prompt and return the result.

### 5.8 DAP launch sequence

```
Client                          maxima-dap                    Maxima
  |                                 |                            |
  |-- initialize ------------------>|                            |
  |<-- initialize response ---------|                            |
  |-- launch(program="file.mac") -->|                            |
  |                                 |-- spawn maxima ----------->|
  |                                 |-- debugmode(true)$ ------->|
  |                                 |-- load("file.mac")$ ------>|
  |<-- initialized event -----------|                            |
  |-- setBreakpoints -------------->|                            |
  |                                 |-- :break f 5 ------------>|
  |<-- breakpoints response --------|                            |
  |-- configurationDone ----------->|                            |
  |                                 |-- evaluate user expr ----->|
  |                                 |<-- dbm:1> (breakpoint) ---|
  |<-- stopped event ---------------|                            |
  |-- stackTrace ------------------>|                            |
  |                                 |-- :bt -------------------->|
  |                                 |<-- backtrace text ---------|
  |<-- stackTrace response ---------|                            |
  |-- continue -------------------->|                            |
  |                                 |-- :resume ---------------->|
  |                                 |<-- normal output ----------|
  |<-- terminated event ------------|                            |
```

### 5.9 Wire into VS Code extension

Add to `package.json`:

```json
{
  "contributes": {
    "debuggers": [{
      "type": "maxima",
      "label": "Maxima Debug",
      "languages": ["maxima"],
      "configurationAttributes": {
        "launch": {
          "required": ["program"],
          "properties": {
            "program": { "type": "string", "description": "Path to .mac file to debug" },
            "maximaPath": { "type": "string", "description": "Path to Maxima binary" },
            "backend": { "enum": ["local", "wsl", "docker"] },
            "stopOnEntry": { "type": "boolean", "default": false }
          }
        }
      },
      "initialConfigurations": [{
        "type": "maxima",
        "request": "launch",
        "name": "Debug Maxima File",
        "program": "${file}"
      }],
      "configurationSnippets": [{
        "label": "Maxima: Launch",
        "description": "Debug a Maxima .mac file",
        "body": {
          "type": "maxima",
          "request": "launch",
          "name": "Debug ${1:file}",
          "program": "^\"\\${workspaceFolder}/${2:main.mac}\""
        }
      }]
    }],
    "breakpoints": [{ "language": "maxima" }]
  }
}
```

In `extension.ts`, register the debug adapter:

```typescript
class MaximaDebugAdapterFactory implements vscode.DebugAdapterDescriptorFactory {
  createDebugAdapterDescriptor(): vscode.DebugAdapterDescriptor {
    const dapPath = config.get<string>('maxima.dap.path') || context.asAbsolutePath('bin/maxima-dap');
    return new vscode.DebugAdapterExecutable(dapPath);
  }
}

context.subscriptions.push(
  vscode.debug.registerDebugAdapterDescriptorFactory('maxima', new MaximaDebugAdapterFactory())
);
```

### 5.10 Deliverables

- [ ] `maxima-dap` crate with stdio binary
- [ ] DAP message types and JSON-RPC framing
- [ ] Launch: spawn Maxima with `debugmode(true)`, load file
- [ ] Breakpoint setting with file:line → function+offset mapping
- [ ] Breakpoint verification (verified/unverified with messages)
- [ ] `dbm:` prompt detection and mode switching
- [ ] Continue, step over, step into
- [ ] Stack trace with source mapping
- [ ] Variable inspection (locals, globals, expandable lists/matrices)
- [ ] Debug console evaluation
- [ ] VS Code extension updated with DAP config and launch.json snippets
- [ ] Integration tests with a mock Maxima process

---

## Phase 6: Aximar Integration

**Goal:** Bidirectional link between the notebook and the editor.

### 6.1 "Open in Aximar" command

From VS Code, open the current `.mac` file in an Aximar notebook. The extension invokes Aximar's MCP server (if running) via the streamable HTTP endpoint, or launches Aximar with the file path as an argument.

### 6.2 LSP for notebook cells

Aximar's notebook cells can use the LSP server for autocomplete and hover, replacing or supplementing the current direct catalog matching. The Tauri app would either:
- Spawn `maxima-lsp` as a subprocess and communicate via stdio
- Or use the LSP server as a library (link `maxima-lsp` crate directly)

Library mode avoids spawning a separate process and shares the catalog in-process.

### 6.3 Notebook export with cell markers

Add "Export to .mac" in Aximar that writes code cells to a `.mac` file with `/* %% */` markers between cells. This file is editable in VS Code with the cell code lens providers from Phase 1.

### 6.4 Deliverables

- [ ] "Open in Aximar" VS Code command
- [ ] LSP-powered completions in Aximar notebook cells
- [ ] ".mac export" from Aximar with cell markers

---

## Binary Distribution

The extension needs to ship `maxima-lsp` and `maxima-dap` binaries for each platform. Options:

### Option A: Bundled binaries (recommended for v1)

Compile for `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`. Include in the `.vsix` package under `bin/`. Extension selects the correct binary at activation based on `process.platform` and `process.arch`.

**Pros:** Works offline, no extra install step.
**Cons:** Large `.vsix` (~40MB with 4 platform binaries).

### Option B: Download on first activation

Extension downloads the correct binary from GitHub releases on first activation. Store in `globalStoragePath`.

**Pros:** Small `.vsix`.
**Cons:** Requires internet, first-run delay.

### Option C: Platform-specific extensions

Publish separate extensions per platform (`maxima-extension-linux-x64`, etc.). VS Code Marketplace supports this natively.

**Pros:** Small per-platform `.vsix`, works offline.
**Cons:** More CI/CD complexity.

**Recommendation:** Start with Option A for simplicity. Move to Option C when the extension is mature enough for the marketplace.

---

## Testing Strategy

### Unit tests

- **maxima-mac-parser:** Parse `.mac` snippets, verify extracted symbols, spans, edge cases. No Maxima process needed.
- **maxima-lsp:** Test completion, hover, and symbol resolution against the catalog. Mock document state.
- **maxima-dap:** Test breakpoint mapping logic. Test DAP message serialization.

### Integration tests

- **maxima-lsp:** Start the server, send LSP requests via JSON-RPC, verify responses. Use the `tower-lsp` test utilities or a custom test harness.
- **maxima-dap:** Start the server with a real Maxima process, set breakpoints, step through code, verify stack frames. Mark as `#[ignore]` like existing integration tests (requires Maxima installed).
- **Extension (e2e):** VS Code extension test runner (`@vscode/test-electron`) — open a `.mac` file, verify completions appear, verify breakpoints set.

### CI

- `cargo test --workspace` covers all unit tests
- `cargo test --workspace -- --ignored` runs integration tests (requires Maxima)
- GitHub Actions matrix: macOS, Linux, Windows (same as existing CI)
- Extension tests: separate workflow using `xvfb-run` on Linux

---

## Dependency Summary

### New Rust crates

| Crate | Key dependency | Purpose |
|-------|---------------|---------|
| `maxima-mac-parser` | (none) | Parse `.mac` files |
| `maxima-lsp` | `tower-lsp 0.20`, `aximar-core`, `maxima-mac-parser` | Language server |
| `maxima-dap` | `aximar-core`, `maxima-mac-parser` | Debug adapter |

### Extension npm packages

| Package | Purpose |
|---------|---------|
| `vscode-languageclient` | LSP client for VS Code |
| `@types/vscode` | TypeScript types |
| `esbuild` | Bundle TypeScript for extension |

---

## Risk Mitigations

| Risk | Mitigation |
|------|------------|
| tower-lsp API instability | Pin to exact version, wrap in a thin adapter layer |
| Maxima debugger unreliability on GCL | Detect Lisp impl at startup, warn user, document SBCL requirement |
| `.mac` parser false positives | Fault-tolerant design — partial results always returned |
| Binary size | Strip symbols, use `lto = true` in release profile |
| Cross-platform path issues | Reuse aximar-core's `Backend` path translation (already handles WSL/Docker) |
| DAP spec complexity | Implement minimal subset first (launch, breakpoints, stepping, stack trace, variables) |
