# Maxima Communication Protocol

How Aximar communicates with the Maxima subprocess.

## Process Lifecycle

```
App Launch → spawn Maxima → init sequence → READY
                                              ↓
User executes cell → send expression → read until sentinel → parse → return result
                                              ↓
                                         (repeat)
                                              ↓
App Close → kill process
```

## Spawning

```bash
maxima --very-quiet
```

The `--very-quiet` flag suppresses the startup banner and version info.

**Binary detection order**:
1. `AXIMAR_MAXIMA_PATH` environment variable
2. Common paths: `/opt/homebrew/bin/maxima`, `/usr/local/bin/maxima`, `/usr/bin/maxima`
3. Windows: `C:\maxima-*\bin\maxima.bat`
4. Fall back to `maxima` (let OS PATH resolve)

## Initialization Sequence

After spawning, send these commands (terminated with `$` for silent execution):

```
display2d:false$
set_plot_option([run_viewer, false])$
set_plot_option([gnuplot_term, svg])$
print("__AXIMAR_READY__")$
```

| Command | Purpose |
|---------|---------|
| `display2d:false` | Output expressions in 1D text form instead of 2D ASCII art |
| `set_plot_option([run_viewer, false])` | Don't open gnuplot viewer windows |
| `set_plot_option([gnuplot_term, svg])` | Output plots as SVG |
| `print("__AXIMAR_READY__")` | Sentinel to confirm init is complete |

Read stdout until `__AXIMAR_READY__` appears. The process is then ready for evaluations.

## Evaluation Protocol

### Sending

For each cell execution, write this sequence to Maxima's stdin:

```
set_plot_option([gnuplot_out_file, "/tmp/aximar_plots/<cell_id>.svg"])$
<user_expression>
tex(%);
print("__AXIMAR_EVAL_END__");
```

| Line | Purpose |
|------|---------|
| `set_plot_option(...)` | Direct any plot output to a unique file for this cell |
| `<user_expression>` | The user's Maxima code, verbatim |
| `tex(%)` | Convert the last result to LaTeX |
| `print("__AXIMAR_EVAL_END__")` | Sentinel marking end of output |

### Receiving

Read stdout line-by-line. Collect all lines until one contains `__AXIMAR_EVAL_END__`.

### Example Session

**Input** (sent to stdin):
```
integrate(x^2, x);
tex(%);
print("__AXIMAR_EVAL_END__");
```

**Output** (read from stdout):
```
x^3/3                          ← 1D text result
$${{x^3}\over{3}}$$            ← LaTeX from tex(%)
false                          ← return value of tex()
__AXIMAR_EVAL_END__            ← sentinel
"__AXIMAR_EVAL_END__"          ← return value of print()
```

## Output Parsing Rules

Given the collected lines (before and including sentinel):

1. **LaTeX extraction**: Lines matching regex `^\$\$.*\$\$$` → extract content between `$$` delimiters
2. **Error detection**: Lines containing ` -- an error.` or starting with `incorrect syntax:`
3. **Filter out**:
   - The line `false` immediately after a LaTeX line (return value of `tex()`)
   - Lines containing `__AXIMAR_EVAL_END__` (sentinel)
   - Lines containing `"__AXIMAR_EVAL_END__"` (print return value)
4. **Plot detection**: After parsing, check if `<plot_dir>/<cell_id>.svg` file exists with recent modification time → read SVG content
5. **Remaining lines**: plain text result

## Parsed Result Structure

```rust
pub struct EvalResult {
    pub cell_id: String,
    pub text_output: String,        // Filtered text lines
    pub latex: Option<String>,      // LaTeX without $$ delimiters
    pub plot_svg: Option<String>,   // SVG file content if plot was produced
    pub error: Option<String>,      // Error message if detected
    pub is_error: bool,
    pub duration_ms: u64,
}
```

## Error Examples

### Division by zero
```
expt: undefined: 0 to a negative exponent.
 -- an error. To debug this try: debugmode(true);
```

### Syntax error
```
incorrect syntax: Premature termination of input at ;.
```

### Unbound variable
```
- Loss of 14 significant digits
```

## Timeout Handling

- Default timeout: 30 seconds per evaluation
- If sentinel not received within timeout:
  1. Kill the Maxima process
  2. Return an error result: "Evaluation timed out after 30 seconds"
  3. Automatically restart Maxima process
  4. Notify frontend of restart

## Crash Recovery

- Periodically check `child.try_wait()` to detect unexpected exit
- On crash: restart process, send init sequence, notify frontend
- Frontend should show "Session restarted" in status bar

## Concurrency

Maxima is single-threaded. Evaluations are serialized via a Mutex on the process handle. If the user queues multiple cells:

1. First cell acquires the lock, evaluates
2. Subsequent cells wait for the lock
3. Frontend shows "queued" status for waiting cells

## Interrupting Evaluation

To cancel a long-running computation:

1. Send SIGINT to the Maxima process (Unix) or equivalent (Windows)
2. If that doesn't work within 2 seconds, kill and restart
3. Return an error result: "Evaluation interrupted"

## LaTeX Preprocessing for KaTeX

Maxima's `tex()` output needs minor adjustments for KaTeX:

| Maxima output | KaTeX-compatible |
|---------------|-----------------|
| `\over` | Works in KaTeX (keep as-is) |
| `\it ` | Replace with `\mathit{` |
| `\pmatrix{...}` | Replace with `\begin{pmatrix}...\end{pmatrix}` |

Build up the preprocessing function as edge cases are discovered during testing.
