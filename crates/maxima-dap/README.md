# maxima-dap

Debug Adapter Protocol (DAP) server for Maxima `.mac` files. Provides interactive debugging in VS Code and any DAP-capable editor — breakpoints, stepping, call stack, and variable inspection.

Requires Maxima with the **SBCL** Lisp backend.

## Features

- **Breakpoints** — Set breakpoints on lines inside function definitions, automatically mapped to Maxima's function+offset format
- **Step Over (F10)** — Advance to the next statement in the current function
- **Step Into (F11)** — Step into sub-expressions and function calls
- **Continue (F5)** — Resume execution until the next breakpoint or completion
- **Stack Trace** — View the call stack with source file and line information
- **Variables** — Inspect function arguments and `block()` local variables at each stack frame
- **Debug Console** — Evaluate arbitrary Maxima expressions while stopped at a breakpoint

## Building

```sh
cargo build --release -p maxima-dap
```

To install on your PATH:

```sh
cargo install --path crates/maxima-dap
```

## Usage

`maxima-dap` communicates via JSON-RPC over stdin/stdout using the DAP Content-Length framing protocol. Your editor starts it as a subprocess — you don't run it directly.

With the [Maxima VS Code extension](https://github.com/cmsd2/maxima-extension), the server is found automatically if it's on your PATH. You can also set `maxima.dap.path` in VS Code settings.

Add a launch configuration in `.vscode/launch.json`:

```json
{
    "type": "maxima",
    "request": "launch",
    "name": "Debug Maxima File",
    "program": "${file}",
    "evaluate": "my_function(args)"
}
```

The file's top-level code (loads, settings, globals) runs first, then `evaluate` if it's set.

Enable debug logging with:

```sh
RUST_LOG=maxima_dap=debug maxima-dap
```

## How it works

The DAP server bridges VS Code's file:line breakpoints to Maxima's function+offset debugger commands (`:break funcname offset`). It parses your `.mac` file with `maxima-mac-parser` to determine which function contains each breakpoint line and computes the offset from the function body start.

To preserve breakpoints across file loading, the server splits your file into two parts:
1. **Definitions** — extracted into a temp file and loaded via `batchload` (blank lines preserve line numbers)
2. **Top-level statements** — evaluated in a `block()` wrapper with terminators converted to commas

## Examples

The [`examples/`](examples/) directory contains 15 `.mac` files covering various debugging scenarios. See [`examples/README.md`](examples/README.md) for a guide.

## Architecture

```
crates/maxima-dap/src/
├── main.rs         # Binary entry point — tracing setup, stdio transport
├── lib.rs          # Module exports
├── transport.rs    # Content-Length framing over stdin/stdout
├── server.rs       # DapServer — request dispatch, state machine, Maxima communication
├── breakpoints.rs  # file:line ↔ function+offset mapping
├── frames.rs       # Backtrace parsing → DAP StackFrame, variable extraction
└── types.rs        # Launch arguments, debug state, mapped breakpoint types
```

## Known limitations

- **SBCL required** — Stack traces and variable inspection only work with the SBCL backend
- **Breakpoints only inside functions** — Top-level lines are marked unverified
- **Step Out stops at statements** — Maxima has no step-out of its own; the server steps until it's back in a caller, and stops at the caller's next statement, or runs on if there's none
- **Some lines can't stop** — With stock Maxima, a breakpoint on a line that starts with arithmetic or holds only a value moves to the function's next line that can stop, or is marked unverified
- **Built-in name conflicts** — Names like `factorial` cannot be redefined; use e.g. `my_factorial`
- **`errcatch` suppresses breakpoints** — Breakpoints inside `errcatch()` blocks may not fire

See the full documentation at [`docs/maxima-dap.md`](../../docs/maxima-dap.md).
