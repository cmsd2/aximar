# maxima-dap

Debug Adapter Protocol (DAP) server for Maxima `.mac` files. Provides interactive debugging in VS Code (and any DAP-capable editor) — breakpoints, stepping, call stack, and variable inspection.

Requires a running Maxima installation with **SBCL** as the Lisp backend. See [Known Limitations](#known-limitations) for details.

## Features

| Feature | Description |
|---------|-------------|
| **Breakpoints** | Set breakpoints on lines inside function definitions. Automatically mapped to Maxima's function+offset format. |
| **Step Over** | Advance to the next statement in the current function (`:next`). |
| **Step Into** | Step into sub-expressions and function calls (`:step`). |
| **Continue** | Resume execution until the next breakpoint or program completion (`:resume`). |
| **Stack Trace** | View the call stack with source file and line information. |
| **Variables** | Inspect function arguments and `block()` local variables at each stack frame. |
| **Debug Console** | Evaluate arbitrary Maxima expressions at the debugger prompt. |

## Building

From the workspace root:

```sh
cargo build --release -p maxima-dap
```

The binary is at `target/release/maxima-dap`.

To install it on your PATH:

```sh
cargo install --path crates/maxima-dap
```

## Running

`maxima-dap` communicates via JSON-RPC over **stdin/stdout** using the DAP Content-Length framing protocol. You don't run it directly — your editor starts it as a subprocess.

Logging goes to **stderr** and is controlled by the `RUST_LOG` environment variable:

```sh
# Default level is info; debug shows all Maxima communication
RUST_LOG=maxima_dap=debug maxima-dap
```

The `debug` level is recommended during development — it logs every command sent to Maxima, every response received, debugger prompt transitions, and breakpoint mapping decisions.

## How It Works

### Architecture

```
VS Code                    maxima-dap                     Maxima (SBCL)
  |                            |                              |
  |-- initialize ------------->|                              |
  |-- launch(program) -------->|-- spawn maxima             ->|
  |                            |-- debugmode(true)$         ->|
  |                            |-- batchload("file.mac")$   ->|
  |-- setBreakpoints --------->|-- :break func offset       ->|
  |-- configurationDone ------>|-- evaluate expression      ->|
  |                            |<- (dbm:1) breakpoint hit   --|
  |<-- stopped event ----------|                              |
  |-- stackTrace ------------->|-- :bt                      ->|
  |<-- stack frames -----------|<- backtrace text           --|
  |-- next ------------------->|-- :next                    ->|
  |<-- stopped event ----------|<- (dbm:1) next prompt      --|
  |-- continue ---------------->|-- :resume                 ->|
  |<-- terminated event --------|<- expression completes    --|
```

### Launch Sequence

The DAP launch sequence is carefully ordered to ensure breakpoints work:

1. **`launch`** — Spawn Maxima, enable `debugmode(true)`, parse the `.mac` file for breakpoint mapping. The file is **not loaded yet**.
2. **`setBreakpoints`** — Map source lines to function+offset pairs. Breakpoints are stored as pending (functions don't exist yet).
3. **`configurationDone`** — Load the file via a temp file that contains **only function/macro definitions** (extracted by `maxima-mac-parser`). Blank lines are inserted to preserve original line numbers so that Maxima's line info matches the source file. Then set pending breakpoints in Maxima (functions now exist). Finally:
   - If an `evaluate` expression is provided: evaluate it (breakpoints fire on function calls).
   - If no `evaluate`: extract and re-execute only the top-level non-definition code from the file. Statement terminators (`$` and `;`) are converted to commas using `maxima_mac_parser::replace_terminators()` so the code can be wrapped in a `block()` for sentinel-based completion detection. This avoids redefining functions (which would clear breakpoints) while still running the file's top-level statements.
   - If the file has no top-level code: terminate with a message suggesting an `evaluate` expression.

If `batchload()` fails (e.g., due to a built-in name conflict), the server detects the error debugger prompt, escapes with `:top`, and terminates the session with an error message.

### Breakpoint Mapping

Maxima's debugger does not support reliable file:line breakpoints. It uses **function name + line offset** instead (`:break funcname offset`). The DAP server bridges this gap:

1. When the user sets a breakpoint at `file.mac:14`, the server parses the file with `maxima-mac-parser`.
2. It finds which function definition contains line 14.
3. It computes the offset from the function's body start line: `offset = line_0based - body_start_line`.
4. It sends `:break funcname offset` to Maxima.
5. When Maxima hits the breakpoint and produces a backtrace, the `:bt` output includes actual 1-based file line numbers, which are used directly for source highlighting in the editor.

#### Example

Given this file:

```maxima
/* lines 1-11: comments */

add(a, b) := block(     /* line 12 — body_start_line = 11 (0-based) */
    [result],            /* line 13 */
    result : a + b,      /* line 14 */
    result               /* line 15 */
)$                       /* line 16 */
```

- Breakpoint on **line 12** → `:break add 0` (offset 0, function entry)
- Breakpoint on **line 14** → `:break add 2` (offset 2)

Lines outside any function definition cannot have breakpoints. The DAP server marks these as **unverified** with a message explaining why.

### Sentinel-Based Communication

The DAP server uses a sentinel protocol to distinguish between "expression completed" and "debugger prompt hit":

```maxima
block([__dap_r__], __dap_r__: (user_expression), print("__SENTINEL__"), __dap_r__)$
```

The sentinel `print()` is embedded **inside** the wrapping block, so it only fires when the expression runs to completion. If a breakpoint fires during evaluation, Maxima switches to the `(dbm:N)` prompt before reaching the sentinel. This lets the server reliably detect both outcomes using chunk-based reading of stdout.

Debugger commands (`:next`, `:step`, `:resume`, `:bt`) do not inject additional sentinels — they reuse the sentinel from the original expression. When the expression eventually completes (after any number of step/resume cycles), the sentinel fires and the server detects normal completion.

### Prompt Detection

Maxima's debugger prompt format is `(dbm:N)` where `N` is the nesting level. This prompt does **not** end with a newline, so the server uses chunk-based `AsyncReadExt::read()` rather than line-based reading to detect it. The regex used is:

```
\(dbm:(\d+)\)
```

### State Machine

The server tracks its state to handle commands appropriately:

```
Uninitialized → Initialized → Running → Stopped{level} → Terminated
                                  ↑          |
                                  └──────────┘
                               (resume/next/step)
```

- **Running:** Expression is being evaluated. Waiting for either a debugger prompt or sentinel.
- **Stopped:** At a `(dbm:N)` prompt. Can accept debugger commands, stack trace requests, and variable inspection.
- **Terminated:** Expression completed (sentinel seen). Debug session is over.

Breakpoint setting is state-aware: at a normal prompt, breakpoints use sentinel-based reading; at a debugger prompt, they use debugger-prompt reading.

## Stepping Behavior

Understanding how stepping works is essential to using the debugger effectively.

### Step Over (`:next`)

`:next` advances to the next statement **at the same nesting level** in the current function. Its behavior depends on where the current execution point is:

- **Function entry (offset 0):** `:next` steps to the next statement in the function body (e.g., from the variable declaration to the first assignment). The debugger stays stopped and you can step again.
- **Middle of function:** `:next` advances to the following statement. Sub-expressions and function calls are evaluated without stopping.
- **Last statement:** `:next` executes the final expression and **exits the function**. If no further breakpoints are hit, the entire evaluation completes and the debug session terminates.

This is standard Maxima debugger behavior and is inherent to how `:next` works — it cannot stop "after" the last statement because there is no next statement to stop at.

#### Practical implications

In a short function like:

```maxima
add(a, b) := block(
    [result],
    result : a + b,
    result
)$
```

If you set a breakpoint on `result : a + b` (offset 2), clicking Step Over will immediately complete the function because `result` (the return value) is the only remaining statement. The program runs to completion.

**For a better stepping experience,** set breakpoints at the start of longer functions (the function definition line), which maps to offset 0. This gives you the most room to step through the body.

In a longer function like:

```maxima
compute(x) := block(
    [a, b, c],
    a : x,            /* offset 2 — step here */
    b : (x + 1)^2,    /* offset 3 — and here */
    c : x + 2,        /* offset 4 — and here */
    a + b + c          /* offset 5 — last :next exits */
)$
```

Setting a breakpoint at the function entry and using Step Over walks through `a`, `b`, `c` assignments one at a time. The final `:next` (at `a + b + c`) exits the function.

### Step Into (`:step`)

`:step` descends into every sub-expression, including individual operator applications. This is much finer-grained than Step Over — for `b : (x + 1)^2`, Step Into will stop at `x + 1`, then `(...)^2`, then the assignment `b : ...`. This is useful for inspecting intermediate values but can be slow for complex expressions.

### Continue (`:resume`)

`:resume` runs until the next breakpoint is hit or the expression completes. There is no "step out" command in Maxima's debugger — `:resume` is the closest equivalent (it continues to the next breakpoint, which may be in a calling function if one is set).

## Maxima Version Compatibility

The DAP server supports two modes, auto-detected at launch:

| Mode | Maxima Version | Detection |
|------|---------------|-----------|
| **Legacy** | Stock Maxima | Default fallback |
| **Enhanced** | Patched Maxima with `set_breakpoint` | Probes for `(fboundp 'maxima::$set_breakpoint)` |

### Legacy mode

Uses function+offset breakpoints (`:break func N`), a temp file for definitions-only loading, and top-level code extraction. This is the original behavior and works with any Maxima installation.

### Enhanced mode

Uses file:line breakpoints (`:break "file.mac" LINE`), deferred breakpoints (set before file is loaded), line-snapping (breakpoints on non-executable lines are adjusted), and direct batchload of the original file (no temp file). Requires a patched Maxima with the enhanced debugger from the `breakpoints-proposal`.

**Enhanced mode benefits:**
- **Deferred breakpoints** — Set breakpoints before loading, auto-resolve when functions are defined
- **Line-snapping** — Breakpoints on comment/blank lines snap to the nearest executable line
- **Path normalization** — File paths are normalized via `probe-file`
- **Breakpoint survival** — Breakpoints are auto-reapplied when functions are redefined
- **Simpler protocol** — No temp file, no function+offset mapping, no top-level code extraction

## Known Limitations

### SBCL required

The Maxima debugger features (`:bt`, `:frame`, line info in backtraces) only work correctly with the **SBCL** Lisp backend. On GCL, `:bt` produces no output and line information is missing. The DAP server detects the backend at launch and warns if it's not SBCL.

### Breakpoints only work inside functions

Maxima's debugger can only set breakpoints on function bodies. In Legacy mode, this uses `:break funcname offset`; in Enhanced mode, `:break "file" LINE`. In both modes, lines at the top level of a `.mac` file (outside any function or macro definition) cannot have breakpoints. The DAP server marks these as **unverified** with the message "Line N is not inside a function definition" (Legacy) or an appropriate error from Maxima (Enhanced).

### No Step Out

Maxima's debugger has no native step-out command. The closest alternatives are:
- `:resume` — continues to the next breakpoint (which may be in a different function)
- `:next` — if you're at the last statement, this exits the current function

### `errcatch` suppresses breakpoints

Breakpoints inside `errcatch()` blocks do not fire. This is a Maxima limitation — `errcatch` catches all interrupts including debugger breaks.

### Function redefinition clears breakpoints (Legacy only)

In Legacy mode, redefining a function (via `batchload()`, `load()`, or re-evaluating its `:=` definition) clears all breakpoints on that function. The DAP server works around this by extracting and re-executing only non-definition top-level code when no `evaluate` expression is provided, so function definitions (and their breakpoints) remain intact.

In Enhanced mode, breakpoints are auto-reapplied when functions are redefined.

### Built-in name conflicts

Certain names like `factorial` (which Maxima maps to the `!` operator) cannot be used as user-defined function names. Attempting to redefine a built-in operator causes `batchload` to fail with an error like `define: function name cannot be a built-in operator or special symbol`. The DAP server detects this and terminates the session cleanly with an error message. Rename your function (e.g., `my_factorial` instead of `factorial`) to avoid the conflict.

### Single-threaded

Maxima is single-threaded. The DAP server always reports a single thread. All stepping and evaluation is synchronous.

### `batchload` vs `load` for line info

The DAP server uses `batchload()` to load program files, which is the standard mechanism for loading files without displaying each result. In rare cases, `batchload()` may fail to preserve line information (producing "No line info" warnings from Maxima). If you encounter this, try using `load()` instead via the launch configuration.

### No hot reload

The debugger does not support modifying source files while a debug session is running. If you change a `.mac` file, you need to restart the debug session for changes to take effect. Breakpoint line numbers will be stale if the file has been edited since the session started.

## Editor Setup

### VS Code

The Maxima VS Code extension registers a debug adapter that spawns `maxima-dap`. Add a `launch.json` configuration:

```json
{
    "version": "0.2.0",
    "configurations": [
        {
            "type": "maxima",
            "request": "launch",
            "name": "Debug Maxima File",
            "program": "${file}",
            "evaluate": "my_function(args)"
        }
    ]
}
```

#### Launch configuration properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `program` | string | yes | Path to the `.mac` file to debug. |
| `evaluate` | string | no | Maxima expression to evaluate after loading the file. If omitted, the file's top-level code is re-executed automatically. |
| `backend` | string | no | Backend type. Currently only `"local"` is supported. Default: `"local"`. |
| `maximaPath` | string | no | Path to the Maxima binary. Default: uses `maxima` from PATH. |
| `stopOnEntry` | boolean | no | Reserved for future use. |

#### Workflow

1. Open a `.mac` file in VS Code.
2. Set breakpoints by clicking in the gutter next to lines **inside function definitions**.
3. Open the Run and Debug panel (`Ctrl+Shift+D`).
4. Select "Debug Maxima File" and press F5.
5. The `evaluate` expression runs and stops at your first breakpoint.
6. Use the debug toolbar: Step Over (F10), Step Into (F11), Continue (F5).
7. Inspect variables in the Variables panel, or type expressions in the Debug Console.

#### Troubleshooting

Enable debug logging by adding to your extension settings or launch config environment:

```json
{
    "env": {
        "RUST_LOG": "maxima_dap=debug"
    }
}
```

### Debug Console output

The Debug Console shows only user-relevant output. Internal protocol
noise is automatically filtered:

| Filtered out | Reason |
|--------------|--------|
| Maxima labels (`(%i1)`, `(%o1)`) | Implementation detail |
| Sentinel strings (`__MAXIMA_DAP_DONE__`) | Internal framing |
| Debugger prompts (`(dbm:1)`) | VS Code shows state in UI |
| Breakpoint messages (`Bkpt 1 for ...`) | VS Code has its own breakpoint UI |
| Stdin echoes (commands sent to Maxima) | Not user-initiated |
| Bare `done`/`true`/`false` | Noise from `batchload`/`debugmode` |

What **does** appear:
- Output from `print()`, `display()`, and similar Maxima functions
- Error messages and warnings
- Computation results (when printed explicitly)

Set `RUST_LOG=maxima_dap=debug` to see full protocol traces on stderr:
- `send_debugger_command:` — what's sent to Maxima and what comes back
- `handle_next:` — whether Step Over stayed in the debugger or completed
- `set_maxima_breakpoint:` — breakpoint mapping and Maxima's confirmation

Common issues:
- **Breakpoint marked "unverified"** — The line is outside a function definition. Move the breakpoint inside a function body.
- **Step Over immediately terminates** — You're at the last statement in a short function. Set the breakpoint at the function entry line instead, or use a function with more statements. See [Stepping Behavior](#stepping-behavior).
- **No line info in backtrace** — Maxima may not have line info for the function. Ensure the file was loaded correctly and SBCL is the backend.
- **`:bt` returns no frames** — You're likely running GCL, not SBCL. Install SBCL and configure Maxima to use it.

## Testing

### Running the test suite

```sh
# Unit tests (breakpoint mapping, serialization, frames parsing)
cargo test -p maxima-dap

# Integration tests (requires Maxima with SBCL on PATH)
cargo test -p maxima-dap -- --ignored

# All workspace tests
cargo test --workspace
```

### Integration tests

Integration tests spawn an actual Maxima process and exercise the debugger communication path. They are marked `#[ignore]` because they require Maxima to be installed. The test suite covers:

| Test | What it verifies |
|------|-----------------|
| `debugger_prompt_detected_on_breakpoint` | `(dbm:N)` prompt is correctly detected when a breakpoint fires |
| `backtrace_at_breakpoint` | `:bt` output is parsed into structured frames with function names |
| `backtrace_frame_has_source_line` | Frames include source file name and correct 1-based line number |
| `resume_completes_evaluation` | `:resume` continues execution and sentinel fires correctly |
| `step_stays_in_debugger` | `:step` produces another debugger prompt (doesn't exit) |
| `evaluate_at_debugger_prompt` | Arbitrary expressions can be evaluated at the debugger prompt |
| `no_stale_sentinel_after_breakpoint` | Sentinel doesn't leak into debugger commands (embedded correctly) |
| `next_stays_in_debugger` | `:next` from function entry (offset 0) stays in debugger |
| `next_at_last_statement_completes` | `:next` from last statement (offset 2) exits function correctly |
| `next_multi_step_through_function` | Multiple `:next` steps walk through a longer function body |
| `top_level_code_hits_breakpoint` | Breakpoints fire when only top-level code is re-executed (no evaluate) |
| `enhanced_file_line_breakpoint` | (Enhanced) `:break "file" LINE` works after batchload |
| `enhanced_deferred_breakpoint` | (Enhanced) Set before load, fires after batchload |
| `enhanced_breakpoint_count` | (Enhanced) `breakpoint_count()` returns correct value |
| `enhanced_clear_breakpoints` | (Enhanced) `clear_breakpoints()` clears everything |

### Example files

Example `.mac` files for testing are in `crates/maxima-dap/examples/`. These cover various debugging scenarios:

| File | Scenario |
|------|----------|
| `01_basic_breakpoint.mac` | Simple breakpoint and variable inspection |
| `02_stepping.mac` | Step through a multi-statement function body |
| `03_step_into.mac` | Step into nested function calls |
| `04_recursion.mac` | Recursive function with deep call stack |
| `05_loop.mac` | Breakpoints inside loop bodies |
| `06_list_variables.mac` | Compound data structures |
| `07_conditionals.mac` | Branch coverage with `if`/`elseif`/`else` |
| `08_multiple_functions.mac` | Multiple breakpoints across functions |
| `09_symbolic_math.mac` | Symbolic expressions and CAS operations |
| `10_error_handling.mac` | `errcatch` behavior (known limitation) |
| `11_closures_and_lambda.mac` | Higher-order functions and lambdas |
| `12_matrix_operations.mac` | Linear algebra and matrix inspection |
| `13_deep_call_stack.mac` | Deep call chain and stack navigation |
| `14_debug_console_eval.mac` | Debug Console REPL evaluation |
| `15_unverified_breakpoints.mac` | Top-level vs. function code breakpoints |

## Developing

### Source layout

```
crates/maxima-dap/src/
├── main.rs               # Binary entry point — tracing setup, stdio transport
├── lib.rs                # Public module exports
├── transport.rs          # Content-Length framing over stdin/stdout
├── server.rs             # DapServer — request dispatch, state machine, Maxima communication
├── strategy.rs           # BreakpointStrategy trait, StrategyContext, result types
├── strategy_legacy.rs    # LegacyStrategy — function+offset breakpoints, temp file
├── strategy_enhanced.rs  # EnhancedStrategy — file:line breakpoints, deferred, line-snapping
├── breakpoints.rs        # file:line ↔ function+offset mapping using maxima-mac-parser
├── frames.rs             # Backtrace parsing → DAP StackFrame, variable extraction
└── types.rs              # MaximaLaunchArguments, DebugState, MappedBreakpoint, VariableRef
```

### Related crates

| Crate | Role |
|-------|------|
| `aximar-core` | Maxima process spawning, debugger prompt parsing (`debugger` module), output sink trait |
| `maxima-mac-parser` | `.mac` file parser — provides function spans, `body_start_line` for breakpoint offset calculation, `block_locals` for variable inspection, and `replace_terminators()` for safe block wrapping |
| `emmy_dap_types` | DAP type definitions (requests, responses, events) with serde support |

### Key types

- `DapServer` (`server.rs`) — The server instance. Manages transport, Maxima process, breakpoint state, and cached stack frames.
- `SourceIndex` (`breakpoints.rs`) — Caches parsed `MacFile`s for breakpoint mapping.
- `PromptKind` (`aximar-core::maxima::debugger`) — Enum distinguishing `Normal` (sentinel seen) from `Debugger { level }` (at `(dbm:N)` prompt).
- `MappedBreakpoint` (`types.rs`) — Tracks the mapping between a DAP breakpoint (file:line) and its Maxima representation (function+offset).

### Related documentation

- [maxima-debugger-internals.md](maxima-debugger-internals.md) — Reference on Maxima's built-in debugger commands, prompt format, and known issues
- [ide-features.md](ide-features.md) — Architecture overview of LSP and DAP integration
- [ide-implementation-plan.md](ide-implementation-plan.md) — Phase 5 covers the DAP server implementation plan
- [maxima-mac-syntax.md](maxima-mac-syntax.md) — Maxima syntax reference (relevant for writing debuggable `.mac` files)
