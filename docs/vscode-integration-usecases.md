# VS Code Integration: Use Cases

User-facing scenarios for Maxima development in VS Code, powered by Aximar's LSP and DAP servers. This document describes what the workflows look like from the user's perspective, what protocols carry them, and where the known limitations are.

Companion docs: [ide-features.md](ide-features.md) (architecture), [maxima-debugger-internals.md](maxima-debugger-internals.md) (debugger reference).

## Starting Point: Existing Extension

[maxima-extension](https://github.com/yshl/maxima-extension) by OhtaYasuhiro (MIT, actively maintained) provides a minimal VS Code extension with:

- TextMate grammar for syntax highlighting (keywords, operators, constants, strings, nested `/* */` comments, function call detection)
- Language configuration (bracket matching, auto-closing, comment toggling)
- File associations: `.mac`, `.max`, `.wxm`

It has **no code** — purely declarative (package.json + grammar JSON + language config). No LSP, DAP, TypeScript, or runtime dependencies. This is the foundation we build on. The enhancement path is:

1. **Fork and extend the grammar** if gaps are found (e.g., `:lisp` escape, `:=`/`::=` definition operators)
2. **Add `package.json` declarations** for LSP client and DAP configuration, pointing at Aximar's Rust binaries
3. **Add thin TypeScript glue** only where VS Code APIs require it (e.g., code lens providers for cell markers, terminal link handlers for error locations, commands for "Run in Aximar")
4. **Keep the Rust crates as the brains** — the extension is a thin shell that spawns `maxima-lsp` and `maxima-dap` binaries

---

## 1. Editing `.mac` Files

### 1.1 Syntax Highlighting

**Scenario:** A user opens `dynamics.mac` in VS Code. Keywords (`if`, `then`, `for`, `do`, `block`, `lambda`), operators (`:=`, `::`, `::`), comments (`/* */`), strings, special constants (`%pi`, `%e`, `inf`), and statement terminators (`;`, `$`) are all coloured distinctly.

**Protocol:** None — this is a TextMate grammar bundled with the extension. Works offline, no Maxima process needed.

**Edge cases:**
- Maxima's `/* */` comments nest, unlike C. The grammar must handle `/* outer /* inner */ still comment */` correctly. TextMate grammars support this via `begin`/`end` with self-referencing includes.
- The `:lisp` escape switches the rest of the line to Common Lisp syntax. The grammar should embed a Lisp scope for these lines.

### 1.2 Autocomplete for Built-in Functions

**Scenario:** The user types `integ` and gets a completion list showing `integrate(expr, var)`, `integrate(expr, var, lo, hi)`, and `integer_partitions(n)` with signatures and brief descriptions.

**Protocol:** LSP `textDocument/completion`. The server returns items from Aximar's function catalog (2500+ entries) filtered by prefix.

**Details:**
- Completions include the function signature as `detail` and the first paragraph of the manual entry as `documentation` (markdown).
- Completing a function inserts a snippet with tab stops for parameters: `integrate(${1:expr}, ${2:var})`.
- Package functions are included but annotated with their package name (e.g., `lsquares_estimates — requires load("lsquares")`). Selecting one could offer a quick-fix to insert the `load()` call.

### 1.3 Hover Documentation

**Scenario:** The user hovers over `ratsimp` in their code. A tooltip shows the signature, a description, and a usage example from the Maxima manual.

**Protocol:** LSP `textDocument/hover`. The server looks up the symbol in the function catalog and returns markdown.

**Details:**
- For user-defined functions (defined earlier in the file or in a loaded file), hover shows the definition site and parameter names extracted by the `.mac` parser.
- For variables from `values`, hover could show the current binding if a background Maxima session is running.

### 1.4 Signature Help

**Scenario:** The user types `solve(` and a tooltip appears showing `solve(expr, var)` and `solve([eqn1, eqn2, ...], [var1, var2, ...])` with the current parameter highlighted as they type.

**Protocol:** LSP `textDocument/signatureHelp`, triggered by `(` and `,`.

### 1.5 Go-to-Definition and Find References

**Scenario:** The user right-clicks on a call to `hamiltonian(q, p)` and selects "Go to Definition". VS Code jumps to the `hamiltonian(q, p) := ...` definition in `physics.mac`.

**Protocol:** LSP `textDocument/definition` and `textDocument/references`.

**Details:**
- The `.mac` parser extracts function definitions (`f(x) := ...`), variable assignments (`name : value`), and `load()` calls to build a workspace symbol table.
- Cross-file resolution requires following `load()` chains. If `main.mac` does `load("physics.mac")$`, symbols defined in `physics.mac` are reachable.
- For built-in functions, "Go to Definition" could open the relevant Maxima manual section in a webview or external browser.

### 1.6 Document Symbols and Outline

**Scenario:** The user opens the Outline panel and sees a tree of all function definitions, variable assignments, and `load()` imports in the current file. Clicking a symbol jumps to it.

**Protocol:** LSP `textDocument/documentSymbol`.

### 1.7 Live Diagnostics

**Scenario:** The user saves `model.mac`. Red squiggles appear under `diff(x^2 y)` with the message: `diff: expected 2 or 3 arguments, got 1`. The Problems panel lists the error with file and line.

**Protocol:** LSP `textDocument/publishDiagnostics`.

**Details:**
- On save, the LSP server loads the file in a background Maxima process (`batchload("model.mac")`) and captures error output.
- Maxima's error messages include line numbers when files are loaded via `batchload`. The LSP parses these into diagnostic ranges.
- Debounced to avoid hammering Maxima on rapid saves. A 500ms delay after the last save is reasonable.
- The background session is separate from any debug session or notebook — it's a disposable process for validation only.

**Limitations:**
- Maxima stops at the first error in a file. Only the first error is reported per save.
- Some errors only manifest at runtime (e.g., wrong number of arguments to a user-defined function). Static analysis can't catch these without evaluating.
- If the file depends on other files being loaded first, diagnostics may produce false positives. The LSP could detect `load()` calls and pre-load dependencies.

---

## 2. Running Maxima Code

### 2.1 Run File

**Scenario:** The user opens `simulation.mac` and runs the command "Maxima: Run File" from the command palette. An integrated terminal panel shows Maxima loading the file and printing results. Errors appear as clickable links that jump to the source line.

**Protocol:** Terminal-based. The extension spawns `maxima --very-quiet --batch simulation.mac` and pipes output to a VS Code terminal. Error output is parsed for `file:line` patterns and turned into clickable links via terminal link providers.

**Variant — Run in Aximar:** The command "Maxima: Run File in Aximar" opens or focuses the Aximar desktop app and loads the file into a new notebook, giving the user the full notebook experience (LaTeX rendering, plots, interactivity) for the file's output.

### 2.2 Run Selection / Run Cell

**Scenario:** The user selects a block of Maxima code and runs "Maxima: Run Selection". The result appears in an output panel below the editor, with LaTeX rendering.

**Protocol:** LSP custom request or a VS Code output channel. The extension sends the selection to a running Maxima process and displays the result.

**Details:**
- The output panel could render LaTeX (via `tex()`) using KaTeX, matching Aximar's notebook rendering.
- This creates a lightweight REPL experience inside VS Code without switching to Aximar.
- State accumulates across "Run Selection" invocations within a session. A "Restart Session" command resets it.

### 2.3 Cell Markers (Notebook-Style Editing)

**Scenario:** The user places `/* %% */` comment markers in their `.mac` file to delimit cells. VS Code shows "Run Cell" code lenses above each marker. Clicking one evaluates that cell's code in sequence.

**Details:**
- This is the "notebook in a plain file" pattern used by Python extensions (with `# %%`).
- Cells run in a persistent Maxima session, so definitions from earlier cells are available.
- Output appears in a side panel or inline below the cell marker.
- The cell marker syntax (`/* %% */`) is a valid Maxima comment, so the file remains loadable by standard Maxima.

---

## 3. Interactive Debugging

### 3.1 Setting Breakpoints

**Scenario:** The user clicks the gutter next to line 15 in `optimizer.mac` to set a breakpoint. A red dot appears. They then run "Maxima: Debug File".

**Protocol:** DAP `setBreakpoints`.

**What happens behind the scenes:**
1. The DAP server parses `optimizer.mac` to find that line 15 is inside `gradient_step(params, lr)`, defined starting at line 10.
2. It computes the offset: line 15 - line 10 = offset 5.
3. It sends `:break gradient_step 5` to the Maxima debugger.
4. If the line is *not* inside any function definition (top-level code), the breakpoint is marked as "unverified" with a hollow circle and a tooltip explaining that only function breakpoints are supported.

**Breakpoint verification:**
- DAP supports `Breakpoint` events with `verified: true/false`. The server sends `verified: false` for breakpoints it can't map to a function, with a `message` explaining why.
- If the user redefines a function after setting a breakpoint, Maxima clears the breakpoint. The DAP server should detect this and send an updated `Breakpoint` event.

### 3.2 Stepping Through Code

**Scenario:** Execution pauses at the breakpoint. The yellow highlight shows the current line. The user clicks "Step Over" (F10) to advance line by line, watching variables update in the sidebar.

**Protocol:** DAP `next` (step over), `stepIn` (step into), `continue`.

**Mapping:**
| VS Code action | DAP request | Maxima command |
|----------------|------------|----------------|
| Continue (F5) | `continue` | `:resume` |
| Step Over (F10) | `next` | `:next` |
| Step Into (F11) | `stepIn` | `:step` |
| Step Out (Shift+F11) | `stepOut` | `:resume` (to next breakpoint) |
| Stop (Shift+F5) | `disconnect` | `:top` |

**Limitation:** Maxima has no native "step out" command. The DAP server implements it as `:resume`, which continues to the next breakpoint rather than returning from the current function. A more sophisticated implementation could temporarily set a breakpoint at the caller and then resume.

### 3.3 Inspecting the Call Stack

**Scenario:** Execution is paused. The Call Stack panel shows:

```
gradient_step  optimizer.mac:15
train_epoch    optimizer.mac:28
main           optimizer.mac:45
```

The user clicks `train_epoch` to see its local variables.

**Protocol:** DAP `stackTrace`, `scopes`, `variables`.

**What happens:**
1. The DAP server sends `:bt` to Maxima and parses the text output into structured frames.
2. Each frame includes the function name and (where available) the source location, reverse-mapped from function+offset back to file:line.
3. `scopes` returns a "Locals" scope for each frame.
4. `variables` for a frame sends `:frame N` to Maxima and parses the reported local bindings.

**Limitation:** Maxima's `:bt` output varies between Lisp implementations. SBCL produces usable output; GCL often produces nothing. The DAP server should target SBCL and report a clear error on GCL.

### 3.4 Variable Inspection

**Scenario:** The Variables panel shows:

```
Locals
  params  [1.5, -0.3, 2.1]
  lr      0.01
  grad    [0.45, -0.12, 0.88]
```

The user expands `params` to see individual list elements. They can also type expressions in the Debug Console to evaluate them in the current context.

**Protocol:** DAP `variables` (for the panel) and `evaluate` (for the debug console).

**Details:**
- At the `dbm:` prompt, any Maxima expression can be evaluated. The DAP server sends the expression and returns the result.
- List and matrix values can be presented as expandable tree nodes using DAP's `variablesReference` mechanism. The server lazily fetches sub-elements via `part(expr, n)`.
- For large expressions, the server truncates the display and offers "View Full Value" which opens a read-only editor with the complete `grind()` output.

### 3.5 Watch Expressions

**Scenario:** The user adds `norm(grad)` to the Watch panel. Each time execution pauses, the panel updates to show the current value of `norm(grad)`.

**Protocol:** DAP `evaluate` with `context: "watch"`, called automatically at each stop.

### 3.6 Data Breakpoints (Watchpoints)

**Scenario:** The user right-clicks the variable `total_loss` in the Variables panel and selects "Break on Value Change". Execution continues and pauses the next time `total_loss` is assigned.

**Protocol:** DAP `setDataBreakpoints`.

**What happens:**
1. The DAP server sends `setcheck: [total_loss]$ setcheckbreak: true$` to Maxima.
2. When `total_loss` is assigned, Maxima enters the debugger automatically.
3. The DAP server detects the `dbm:` prompt and reports a `stopped` event with reason `"data breakpoint"`.

**Limitation:** `setcheckbreak` is global — it breaks on *any* assignment to any variable in the `setcheck` list, not just the one the user selected. Removing a data breakpoint requires rebuilding the entire `setcheck` list without that variable.

### 3.7 Conditional Breakpoints

**Scenario:** The user right-clicks a breakpoint and adds the condition `iteration > 100`. Execution only pauses when that condition is true.

**Protocol:** DAP `setBreakpoints` with `condition` field.

**What happens:**
- The DAP server uses `trace_options(f, break(lambda([level, direction, func, item], is(iteration > 100))))` to set a conditional break on function entry.
- For line-level conditional breakpoints (not just function entry), this is harder. The server may need to set an unconditional breakpoint and implement the condition check itself: pause, evaluate the condition, and auto-resume if false.

### 3.8 Logpoints

**Scenario:** The user sets a logpoint (diamond marker) at line 20 with the message `"step {n}: loss = {total_loss}"`. Each time execution reaches that line, the message is printed to the Debug Console without pausing.

**Protocol:** DAP `setBreakpoints` with `logMessage` field.

**What happens:**
- The server uses `trace_options(f, info(print("step", n, ": loss =", total_loss)))` to log without breaking.
- Alternatively, it sets a breakpoint, evaluates the log expression, and auto-resumes — transparent to the user.

---

## 4. Profiling and Performance

### 4.1 Function Timing

**Scenario:** The user runs "Maxima: Profile Function" on `expensive_computation`. After running their code, a panel shows:

```
expensive_computation: 342 calls, 1.47s total, 0.0043s avg
  helper_fn:          1026 calls, 0.89s total, 0.00087s avg
```

**Protocol:** Custom LSP request or extension command. Uses Maxima's `timer()` / `timer_info()` under the hood.

**Details:**
- The extension sends `timer(expensive_computation)$`, runs the user's code, then queries `timer_info()`.
- Results are presented in a table view, sortable by total time, call count, or average time.
- A "Clear Timers" command sends `untimer()`.

---

## 5. Notebook and Editor Interop

### 5.1 Open in Aximar

**Scenario:** The user is editing `analysis.mac` in VS Code and wants to explore results interactively. They run "Maxima: Open in Aximar". Aximar launches (or focuses) and loads the file's contents into a new notebook, splitting at cell markers or function boundaries.

**Protocol:** The extension invokes Aximar via CLI or its MCP endpoint. Aximar's MCP server already supports `add_cell` and `run_cell`.

### 5.2 Aximar Notebook to `.mac` File

**Scenario:** The user has built up a working solution in an Aximar notebook and wants to extract it into a `.mac` file for reuse. They run "Export to .mac" in Aximar, which writes all code cells (skipping markdown) to a file with cell markers.

**Protocol:** Aximar feature, not VS Code. But the exported file is immediately editable in VS Code with full LSP support.

### 5.3 Shared Session (Future)

**Scenario:** The user has a Maxima session running in Aximar with variables and functions defined. In VS Code, autocomplete for their `.mac` file includes those live symbols alongside the static catalog.

**Protocol:** The LSP server connects to Aximar's running session (via MCP or a shared session protocol) to query `values` and `functions` for dynamic completions.

**This is speculative.** It requires a session-sharing mechanism between the LSP and Aximar. The simpler alternative is for the LSP to maintain its own background session.

---

## 6. Multi-File Projects

### 6.1 Workspace Symbol Search

**Scenario:** The user presses `Ctrl+T` and types `hamilton`. The symbol search shows `hamiltonian(q, p)` defined in `physics.mac:12` and `hamilton_equations(H, vars)` in `mechanics.mac:45`.

**Protocol:** LSP `workspace/symbol`.

### 6.2 Cross-File Diagnostics

**Scenario:** The user renames a function parameter in `utils.mac` but forgets to update a caller in `main.mac`. On saving `utils.mac`, the LSP re-evaluates `main.mac` (which loads `utils.mac`) and reports the error.

**Protocol:** LSP `textDocument/publishDiagnostics` on dependent files.

**Details:**
- The LSP tracks `load()` dependency graphs. When a file changes, dependents are re-validated.
- This only catches errors that Maxima reports during `batchload`. Subtle semantic errors (e.g., wrong argument count to a user function) aren't caught unless the call is actually evaluated.

### 6.3 Project Configuration

A `.maxima-project.json` or equivalent in the workspace root could specify:

```json
{
  "entryPoint": "main.mac",
  "loadPath": ["lib/", "vendor/"],
  "backend": "sbcl",
  "maximaPath": "/usr/local/bin/maxima"
}
```

This tells the LSP where to find files for cross-file resolution and which Maxima binary to use for diagnostics.

---

## Protocol Summary

| Feature | Protocol | Maxima Process Required |
|---------|----------|------------------------|
| Syntax highlighting | TextMate grammar | No |
| Completions (built-in) | LSP | No (catalog only) |
| Hover docs (built-in) | LSP | No (catalog only) |
| Signature help | LSP | No (catalog only) |
| Document symbols | LSP | No (parser only) |
| Go-to-definition (local) | LSP | No (parser only) |
| Completions (user symbols) | LSP | No (parser only) |
| Diagnostics | LSP | Yes (background) |
| Completions (live session) | LSP | Yes (background) |
| Run file | Terminal | Yes |
| Run selection | Extension + process | Yes |
| Breakpoints | DAP | Yes (debug session) |
| Stepping | DAP | Yes (debug session) |
| Call stack | DAP | Yes (debug session) |
| Variables | DAP | Yes (debug session) |
| Watch expressions | DAP | Yes (debug session) |
| Data breakpoints | DAP | Yes (debug session) |
| Profiling | Extension command | Yes |

---

## Known Limitations and Workarounds

### Maxima debugger requires SBCL

GCL (the default on many Linux distros) has broken backtrace output and unreliable stepping. The extension should detect the Lisp backend on startup (via `:lisp (lisp-implementation-type)`) and warn the user if it's not SBCL.

### No step-out

Maxima's debugger has `:step` (into) and `:next` (over) but no step-out. The DAP server can approximate it by setting a temporary breakpoint at the calling frame and resuming, but this is fragile if the caller is in non-user code.

### File-path breakpoints are unreliable

The DAP must always translate file:line breakpoints to function+offset form. This means breakpoints on top-level code (outside any function) cannot work. The extension should show these as unverified with a clear explanation.

### Single error per file

`batchload` stops at the first error. Users editing a file with multiple errors only see the first one. After fixing it and saving, the next error appears. This is annoying but unavoidable without modifying Maxima.

### `errcatch` suppresses breakpoints

Code wrapped in `errcatch(...)` swallows errors *and* suppresses breakpoints inside the wrapped code. This is a Maxima design decision. The debugger docs should mention this so users aren't confused when breakpoints don't fire inside `errcatch` blocks.

### Dynamic language limitations

Maxima is dynamically typed and dynamically scoped (with lexical scoping via `block([locals], ...)`). The LSP cannot provide the depth of analysis that TypeScript or Rust language servers can. Completions and diagnostics are best-effort based on the catalog, parser, and background evaluation.
