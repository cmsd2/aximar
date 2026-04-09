# IDE Features for Maxima Programming

Design notes for adding developer tooling — editing, debugging, and state inspection — for Maxima `.mac` files and interactive sessions.

## Goals

- Breakpoints, stepping, stack inspection, and variable watch for Maxima code
- Autocomplete, hover docs, go-to-definition, and diagnostics for `.mac` files
- Leverage existing IDE infrastructure (VS Code, Neovim, Emacs) rather than rebuilding it
- No modifications to Maxima's source code
- Reuse aximar-core's process management, protocol layer, and function catalog

## Non-goals

- Turning aximar into a full code editor (it's a notebook; file editing belongs in editors)
- Lisp-level debugging (SBCL+SLIME already does this well)
- Semantic type analysis (Maxima is dynamically typed; diminishing returns)

## Current State of Maxima Developer Tooling

### What exists

| Tool | Status | Notes |
|------|--------|-------|
| Built-in debugger (`mdebug.lisp`) | Works partially | Function-name breakpoints reliable; file-path breakpoints unreliable |
| Emacs `maxima-mode` | Maintained | Syntax highlighting, send-to-REPL, indentation |
| Emacs `imaxima` | Maintained | LaTeX rendering via control-character protocol |
| Emacs `dbl.el` | Unmaintained | Source highlighting on breakpoints via text parsing |
| `realgud-maxima` | Stalled (2017) | Minimal Emacs debugger wrapper |
| VS Code debug adapter | Attempted (2025) | Blocked by file-path breakpoint bugs in Maxima |
| [maxima-extension](https://github.com/yshl/maxima-extension) | Active (v0.0.2) | VS Code syntax highlighting for `.mac`/`.max`/`.wxm` (our starting point) |
| wxMaxima XML protocol | Fragile | Custom XML over TCP; debugging doesn't work through it |
| LSP for Maxima | Proposed (2020) | Never implemented |
| Scintilla syntax highlighting | Exists | Contributed by wxMaxima maintainer |

### Community sentiment (from mailing list)

- Experts recommend `trace()` over the debugger for everyday use
- GCL is generally considered unsuitable for debugging; SBCL is recommended
- Recurring complaints: breakpoints in packages don't fire, `:bt` produces no output on GCL, `errcatch` suppresses breakpoints
- A 2022 thread proposed `--machine-readable` output from Maxima but reached no consensus
- Demand for modern IDE features is clear but unmet

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                 Shared Rust crates                   │
│                                                      │
│  aximar-core     (process, protocol, catalog)        │
│  maxima-lsp      (Language Server Protocol server)   │
│  maxima-dap      (Debug Adapter Protocol server)     │
│                                                      │
├───────────┬───────────┬─────────────────────────────┤
│  aximar   │  VS Code  │  Any LSP/DAP editor          │
│  (Tauri)  │ extension │  (Neovim, Emacs, Zed, ...)   │
│           │           │                              │
│ notebook  │ .mac file │  .mac file editing            │
│ REPL      │ editing   │  + debugging                  │
│ plots     │ debugging │                              │
│ LaTeX     │ run file  │                              │
└───────────┴───────────┴─────────────────────────────┘
```

The key insight: aximar-core already has the hard infrastructure (process spawning, output parsing, function catalog with 2500+ entries, error enhancement). The LSP and DAP servers are new crates in the same workspace that reuse those internals.

### Why separate from aximar

- **aximar is a notebook.** File-based editing with tree views, tabs, and gutter breakpoints is what code editors do. Building that in Tauri means poorly reimplementing VS Code.
- **LSP/DAP are protocol standards.** One implementation serves every editor. A VS Code extension is mostly declarative JSON config pointing at the servers.
- **Aximar can consume these too.** The notebook cells can use the LSP for autocomplete and hover. A "debug this cell" mode could use the DAP internally. But the primary debugging UI lives in editors.

## Component Design

### 1. TextMate Grammar for `.mac` Files

[maxima-extension](https://github.com/yshl/maxima-extension) (MIT, by OhtaYasuhiro) is an existing VS Code extension we use as our starting point. It is purely declarative — no TypeScript, no runtime dependencies — and provides:

- TextMate grammar with: keywords (`if`/`then`/`for`/`do`/`while`/`return`/etc.), logical operators (`and`/`or`/`not`), `define` and `load` keywords, nested `/* */` comments, double-quoted strings with escapes, numeric constants (integers, floats, scientific notation with `b`/`d`/`e`/`s` exponents), language constants (`%pi`, `%e`, `%i`, `%phi`, `%gamma`, `inf`, `minf`, etc.), generic function-call detection (identifier before `(`), and variable patterns including `%`, `%%`, `%iN`/`%oN`, `_`, `__`.
- Language configuration: block comment toggling, bracket matching/auto-closing for `()`, `[]`, `{}`, `""`.
- File associations: `.mac`, `.max`, `.wxm`.

#### Grammar gaps to address

The existing grammar covers the core well but has gaps for Maxima-specific constructs:

| Missing | Scope needed | Notes |
|---------|-------------|-------|
| `:=` and `::=` operators | `keyword.operator.definition` | Function/macro definition — visually distinct from `:` assignment |
| `block`, `lambda` | `keyword.control` | Currently unhighlighted |
| `;` vs `$` terminators | `punctuation.terminator` | Could use distinct scopes to visually distinguish display vs suppress |
| `:lisp ...` escape | Embedded Lisp scope | Rest of line switches to Common Lisp syntax |
| `:` (assign), `::` (eval-assign) | `keyword.operator.assignment` | Currently no operator highlighting |
| Definition-site functions | `entity.name.function` | `f(x) := ...` should highlight `f` differently from call sites |

These are refinements, not blockers. The grammar works today and can be extended incrementally.

Existing references: `docs/maxima-mac-syntax.md` in this repo, wxMaxima's Scintilla lexer.

### 2. maxima-lsp (Language Server)

An LSP server providing editing features for `.mac` files.

#### Capabilities by difficulty

**Straightforward (use existing catalog):**

| Feature | LSP method | Data source |
|---------|-----------|-------------|
| Completions | `textDocument/completion` | Function catalog (2500+ entries) |
| Hover docs | `textDocument/hover` | Catalog signatures + descriptions |
| Signature help | `textDocument/signatureHelp` | Catalog parameter info |

**Moderate (requires `.mac` file parsing):**

| Feature | LSP method | Implementation |
|---------|-----------|----------------|
| Document symbols | `textDocument/documentSymbol` | Parse `f(x) := ...` patterns |
| Go-to-definition (local) | `textDocument/definition` | Symbol table within file/workspace |
| Find references | `textDocument/references` | Grep-level symbol search |
| Folding ranges | `textDocument/foldingRange` | `block(...)`, `if...`, `/* ... */` |
| Bracket matching | Built-in with grammar | Parens, brackets |

**Harder (requires background Maxima process):**

| Feature | LSP method | Implementation |
|---------|-----------|----------------|
| Diagnostics | `textDocument/publishDiagnostics` | Evaluate in side session, parse errors |
| Completion for loaded packages | `textDocument/completion` | Query `functions` after `load()` |
| Dynamic symbol resolution | Various | Query running Maxima for `values`, `functions` |

#### `.mac` file parser

Doesn't need to be a full Maxima parser. A lightweight incremental parser that extracts:

- Function definitions: name, parameters, line range, docstring (comment before def)
- Variable assignments: `name : value`
- `load()` calls: which packages/files are imported
- Block structure: for folding and scope

This can be regex/tree-sitter based. A tree-sitter grammar for Maxima would be ideal (enables incremental parsing, used by Neovim/Helix/Zed natively) but is a larger undertaking.

#### Background Maxima session

For diagnostics, the LSP can maintain a background Maxima process:

- On file save, `batchload("file.mac")` and capture errors
- Parse error output to extract line numbers and messages
- Publish as LSP diagnostics (red squiggles)
- Rate-limit to avoid hammering Maxima on every keystroke

### 3. maxima-dap (Debug Adapter)

A DAP server wrapping Maxima's built-in text-based debugger. See [maxima-dap.md](maxima-dap.md) for full usage documentation, stepping behavior, and known limitations.

#### How Maxima's debugger works

See [maxima-debugger-internals.md](maxima-debugger-internals.md) for full details.

Summary: The debugger is implemented in `src/mdebug.lisp`. When a breakpoint fires or an error occurs (with `debugmode(true)`), Maxima switches to a `dbm:` prompt. All interaction is text-based over the same stdin/stdout pipes.

#### Protocol state machine

The aximar-core protocol layer currently assumes synchronous request/response. Debugging requires a state machine that handles mode switches:

```
                    ┌──────────┐
                    │  Normal  │
                    │  Mode    │
                    └────┬─────┘
                         │ breakpoint hit
                         │ (detect "dbm:" prompt)
                         ▼
                    ┌──────────┐
          :step ──▶ │ Debugger │ ◀── :bt, :frame
          :next ──▶ │  Mode    │ ◀── inspect expr
                    └────┬─────┘
                         │ :resume / :continue
                         ▼
                    ┌──────────┐
                    │  Normal  │
                    │  Mode    │
                    └──────────┘
```

The process reader needs to detect when output contains a `dbm:` prompt and switch to debugger mode, routing subsequent I/O through DAP events rather than the normal eval response path.

#### Breakpoint mapping

**The core challenge:** DAP clients set breakpoints by file path + line number. Maxima's debugger sets breakpoints by function name + line offset. File-path breakpoints (`:break "file.mac" 5`) exist but are unreliable.

**Workaround:**

1. When the user sets a breakpoint at `file.mac:42`, the DAP server parses the file
2. Find which function definition contains line 42
3. Compute the offset from the function's start line
4. Send `:break funcname offset` to Maxima
5. When the breakpoint fires, reverse-map back to file:line for the DAP client

This requires the same `.mac` parser used by the LSP (another reason to share crates).

**Limitations:**
- Breakpoints outside function definitions won't work (top-level code)
- If the file has been modified since `load()`, line numbers may be stale
- Nested `block()` definitions may confuse offset calculation

#### DAP capabilities to implement

| DAP request | Maxima command | Notes |
|-------------|---------------|-------|
| `setBreakpoints` | `:break f n` | Via file:line → function+offset mapping |
| `continue` | `:resume` | |
| `next` | `:next` | Step over |
| `stepIn` | `:step` | Step into |
| `stepOut` | `:resume` to next breakpoint | No native step-out |
| `stackTrace` | `:bt` | Parse text frames |
| `scopes` | `:frame N` | Parse frame variables |
| `variables` | Evaluate expressions | At debugger prompt |
| `evaluate` | Direct expression eval | Debugger accepts arbitrary exprs |
| `disconnect` | `:top` | Return to normal mode |

#### Variable inspection

At the debugger prompt, arbitrary Maxima expressions can be evaluated. For the variables pane:

- `:frame N` shows the frame's local bindings
- Evaluating a variable name returns its current value
- `values` lists all user-assigned globals
- `fundef(f)` shows a function's definition

#### Watch expressions and conditional breakpoints

- **Watch:** `setcheck: [var1, var2]$` monitors assignments; `setcheckbreak: true$` breaks on assignment
- **Conditional breakpoints:** `trace_options(f, break(some_predicate))` breaks conditionally on function entry
- **Logpoints:** `trace_options(f, info(print_something))` logs without breaking

These map naturally to DAP's `setDataBreakpoints`, conditional breakpoints, and logpoints.

### 4. VS Code Extension

We fork [maxima-extension](https://github.com/yshl/maxima-extension) and extend it. The existing extension already provides the file structure:

```
maxima-extension/
  package.json                    # Language contribution (already declared)
  syntaxes/
    maxima.tmLanguage.json        # TextMate grammar (already working)
  language-configuration.json     # Brackets, comments (already working)
```

What we add:

```
  src/                            # TypeScript glue (new)
    extension.ts                  # Activation, LSP/DAP client startup
    commands.ts                   # "Run File", "Run Selection", "Open in Aximar"
    cellCodeLens.ts               # Code lens for /* %% */ cell markers
    terminalLinks.ts              # Clickable error locations in terminal output
  bin/                            # Bundled Rust binaries (new, or downloaded)
    maxima-lsp
    maxima-dap
```

The `package.json` additions:
- `activationEvents`: `onLanguage:maxima`
- `main`: points to compiled TypeScript entry point
- LSP client configuration: spawns `maxima-lsp` binary (stdio transport)
- DAP `debuggers` contribution: spawns `maxima-dap` binary
- Commands: "Maxima: Run File", "Maxima: Run Selection", "Maxima: Open in Aximar"
- Settings: `maxima.maximaPath`, `maxima.backend` (local/wsl/docker), `maxima.lspPath`, `maxima.dapPath`

#### Why fork rather than build from scratch

- The grammar and language config already work and handle edge cases (nested comments, Maxima-specific constants, exponent notation).
- The language ID `maxima` and file associations are already declared correctly.
- MIT license permits forking and extending.
- We avoid duplicating work on TextMate patterns that someone else maintains.

#### What stays in TypeScript vs Rust

**TypeScript (thin glue):** VS Code API interactions that must run in the extension host — LSP/DAP client lifecycle, code lens providers, command registration, settings access, terminal management.

**Rust (all the logic):** Everything that processes Maxima output, manages sessions, parses `.mac` files, maps breakpoints, or queries the catalog. The TypeScript layer never interprets Maxima — it just passes messages to the LSP/DAP binaries over stdio.

## Build Order

### Phase 1: Grammar refinement and extension scaffold (exists → enhance)

The [maxima-extension](https://github.com/yshl/maxima-extension) already provides a working TextMate grammar and language configuration. This phase forks it and fills the grammar gaps listed above (`:=`/`::=`, `block`/`lambda`, `:lisp` escape, definition-site highlighting). Add the TypeScript scaffold (`extension.ts`), `activationEvents`, and basic commands ("Run File" via terminal).

- Deliverable: published VS Code extension with improved syntax highlighting and "Run File" command
- No Rust binaries needed yet — just `maxima --batch` via the terminal

### Phase 2: maxima-lsp with catalog-based features

- Create `maxima-lsp` crate in the workspace, reusing aximar-core's catalog
- Implement completions, hover, signature help using existing catalog data (2500+ entries)
- Add basic `.mac` symbol extraction (function defs, assignments, loads)
- Wire up document symbols and local go-to-definition
- Ship: update VS Code extension with LSP client that spawns the `maxima-lsp` binary

### Phase 3: maxima-lsp with live diagnostics

- Add background Maxima process management (reuse aximar-core's process/protocol layer)
- On save: `batchload("file.mac")`, capture errors, publish diagnostics
- Add completion for symbols from loaded packages
- Cross-file resolution via `load()` dependency tracking

### Phase 4: maxima-dap

- Extend aximar-core process layer with debugger mode detection (`dbm:` prompt)
- Implement the protocol state machine (normal <-> debugger)
- Build the file:line -> function+offset breakpoint mapper (shares `.mac` parser with LSP)
- Implement core DAP requests: setBreakpoints, continue, next, stepIn, stackTrace, variables
- Ship: update VS Code extension with DAP `debuggers` contribution

### Phase 5: Aximar integration

- Use LSP for notebook cell autocomplete (supplement or replace current catalog matching)
- Optional "debug cell" mode using DAP internally
- Bidirectional link: "Open in VS Code" from Aximar, "Open in Aximar" from VS Code
- Notebook export to `.mac` with cell markers that VS Code can run

## Risks and Open Questions

### Maxima debugger reliability

The debugger has known issues:
- File-path breakpoints don't fire reliably
- `:bt` produces no output on GCL (SBCL works)
- `errcatch` suppresses breakpoints
- Breakpoints in some packages never fire

Mitigation: Target SBCL only for debugging features. Document GCL limitations. The function-name breakpoint workaround avoids file-path issues.

### Output parsing fragility

All Maxima communication is unstructured text. The sentinel-based protocol works for normal eval but the debugger adds complexity:
- Need to distinguish normal output, error output, and debugger prompts
- Multi-line expressions in the debugger may interleave with prompt text
- Maxima may print warnings or messages between debugger commands

Mitigation: Use unique sentinel delimiters (as aximar already does). Consider injecting custom prompt strings via `:lisp (setq *prompt-prefix* "<<<AXIMAR:" *prompt-suffix* ">>>")` to make prompt detection robust.

### tree-sitter grammar

A tree-sitter grammar for Maxima would unlock:
- Incremental parsing for LSP
- Syntax highlighting in Neovim, Helix, Zed (natively)
- Structural editing, text objects

But it's a significant undertaking. The TextMate grammar is good enough for Phase 1. A tree-sitter grammar could replace it later.

### Shared session vs. separate sessions

Should the LSP's background Maxima and the DAP's debug Maxima be separate processes?

- **Separate:** Simpler, no state conflicts. LSP can freely load/eval for diagnostics without affecting the debug session.
- **Shared:** Avoids double resource usage. But diagnostics evaluation could interfere with debugger state.

Recommendation: Separate processes. Maxima is lightweight (~20MB RAM). The isolation is worth it.

## Prior Art and References

- [maxima-extension](https://github.com/yshl/maxima-extension) — VS Code syntax highlighting (our starting point, MIT)
- [Debug Adapter Protocol spec](https://microsoft.github.io/debug-adapter-protocol/)
- [Language Server Protocol spec](https://microsoft.github.io/language-server-protocol/)
- [TextMate grammar guide](https://macromates.com/manual/en/language_grammars)
- [tree-sitter docs](https://tree-sitter.github.io/tree-sitter/)
- Emacs `dbl.el` — text-parsing debugger integration (in `maxima/interfaces/emacs/misc/dbl.el`)
- `realgud-maxima` — https://github.com/realgud/realgud-maxima
- wxMaxima XML protocol — `wxmathml.lisp` in wxMaxima source
- Maxima debugger docs — `maxima/doc/info/Debugging.texi`
- aximar protocol doc — `docs/maxima-protocol.md` (this repo)
- [vscode-integration-usecases.md](vscode-integration-usecases.md) — end-user scenarios (companion doc)
