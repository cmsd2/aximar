# Debugging Maxima Files in VS Code

These example `.mac` files show how to use the VS Code debugger with Maxima. Each file demonstrates a different debugging technique — open one, set some breakpoints, and step through to see how it works.

## Prerequisites

- **Maxima** installed and on your PATH. You need the **SBCL** Lisp backend (the default on most installations).
- **VS Code** with the [Maxima extension](https://github.com/cmsd2/maxima-extension) installed.
- The DAP server binary built: `cargo build -p maxima-dap` (or `cargo install --path crates/maxima-dap`).
- In VS Code settings, set `maxima.dap.path` to the built binary (e.g. `<repo>/target/debug/maxima-dap`).

## Getting started

1. Open any `.mac` file from this folder in VS Code.
2. Click in the left gutter next to a line **inside a function body** to set a red breakpoint dot.
3. Press **F5** (or use **Run > Start Debugging**).
4. When the debugger stops at your breakpoint, you can:
   - **Step Over (F10)** — run the current line and move to the next one
   - **Step Into (F11)** — dive into a function call to see what happens inside
   - **Continue (F5)** — run until the next breakpoint or until the program finishes
5. Look at the **Variables** panel (left side) to see argument values and local variables.
6. Type Maxima expressions in the **Debug Console** (bottom panel) to evaluate them interactively.

### How the debugger loads your file

The DAP server doesn't simply `batchload` your entire file — that would redefine functions and clear any breakpoints you've set. Instead, it splits your file into two parts:

1. **Definitions only** — Function and macro definitions (`:=` and `::=`) are extracted into a temporary file and loaded via `batchload`. Blank lines preserve the original line numbers so breakpoints and stack traces point to the right place.
2. **Top-level statements** — Everything else (function calls, `print(...)`, variable assignments outside functions) is collected and evaluated inside a `block(...)` wrapper. Statement terminators (`$` and `;`) are converted to commas automatically.

This means breakpoints survive the loading process, and your top-level code still runs.

### Choosing what to evaluate

Each example file defines functions and then calls them at the bottom. By default, the debugger runs the file's top-level code automatically (step 2 above).

To call a specific function instead, add an `evaluate` field to your launch config (`.vscode/launch.json`):

```json
{
    "type": "maxima",
    "request": "launch",
    "name": "Debug Example",
    "program": "${file}",
    "evaluate": "my_factorial(5)"
}
```

## Examples

Start with the first few examples and work your way up. Each file has a comment at the top explaining what to try.

| File | What you'll learn |
|------|-------------------|
| `01_basic_breakpoint.mac` | Set a breakpoint and inspect variable values |
| `02_stepping.mac` | Step through a function line by line with F10 |
| `03_step_into.mac` | Step into a nested function call with F11 |
| `04_recursion.mac` | Watch a recursive function build up the call stack |
| `05_loop.mac` | Hit a breakpoint repeatedly inside a loop |
| `06_list_variables.mac` | Inspect lists and compound data structures |
| `07_conditionals.mac` | Step through `if`/`elseif`/`else` branches |
| `08_multiple_functions.mac` | Set breakpoints in several functions at once |
| `09_symbolic_math.mac` | Debug symbolic (non-numeric) expressions |
| `10_error_handling.mac` | See how `errcatch()` affects breakpoints |
| `11_closures_and_lambda.mac` | Debug higher-order functions and lambdas |
| `12_matrix_operations.mac` | Inspect matrix values while debugging |
| `13_deep_call_stack.mac` | Navigate a deep call stack in the Stack Trace panel |
| `14_debug_console_eval.mac` | Evaluate expressions in the Debug Console while stopped |
| `15_unverified_breakpoints.mac` | Understand why breakpoints outside functions appear grey |

## Tips

- **Breakpoints only work inside functions.** If you set a breakpoint on a line that's not inside a function body (like a top-level `print(...)` call), it will appear grey and won't stop execution.
- **Avoid built-in names.** Some names like `factorial` are reserved by Maxima (it maps to the `!` operator internally). If the debugger reports an error loading your file, try renaming your function — e.g., use `my_factorial` instead of `factorial`.
- **Step Over at the last line exits the function.** If there's only one statement left, pressing F10 will finish the function and may end the debug session. Set breakpoints at the function entry line for more stepping room.

## Troubleshooting

- **Breakpoint shows grey (unverified):** The line is outside a function body. Move the breakpoint inside a function.
- **"Error loading definitions":** Your file may use a reserved Maxima name. Check the error message and rename the conflicting function.
- **No variables visible:** Make sure you're stopped at a breakpoint (the yellow arrow in the gutter), not just paused.
- **Step Over immediately ends:** You're at the last statement in the function. See the tip above.

## Known limitations

- **SBCL required** — Stack traces and variable inspection only work with the SBCL Lisp backend, not GCL or ECL.
- **No step-out** — Use Continue (F5) to run to the next breakpoint instead.
- **`errcatch` suppresses breakpoints** — Breakpoints inside `errcatch()` blocks may not fire.
- **Function redefinition** — Reloading a file clears breakpoints on redefined functions. The debugger avoids this by loading only definitions, but it's worth knowing.
