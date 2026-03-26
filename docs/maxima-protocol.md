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

## Backend Modes

Aximar supports three backends for running Maxima, configured in Settings:

| Backend | How it runs | When to use |
|---------|------------|-------------|
| **Local** (default) | Spawns a local `maxima` process directly | Maxima installed natively |
| **Docker** | Runs Maxima inside a container via `docker run` or `podman run` | Easy setup on Windows/Linux; sandboxed execution |
| **WSL** | Runs Maxima inside a WSL distribution via `wsl -d <distro> -- maxima` | Windows with WSL2 and Maxima installed in a Linux distro |

### Docker backend

- Container engine: `docker` or `podman` (argument-compatible)
- Runs with `--rm -i --network none --memory 512m` for isolation
- A host temp directory is volume-mounted to `/tmp/aximar` inside the container so SVG plots are accessible to the host
- A `maxima_tempdir` command is sent during initialization so gnuplot writes SVGs to the mounted volume
- On process kill, `<engine> rm -f <container>` is run as a safety net

A minimal Docker image is provided in `docker/Dockerfile`:

```bash
docker build -t aximar/maxima docker/
```

### WSL backend

- Uses `wsl -d <distro> -- maxima --very-quiet`
- If no distro is specified, uses the default WSL distribution
- SVG paths are translated from `/tmp/...` to `\\wsl.localhost\<distro>\tmp\...`

### Preflight checks

Before spawning, each backend runs a preflight check:
- **Docker/Podman**: verifies `<engine> info` succeeds and the configured image exists
- **WSL**: verifies `wsl --status` succeeds and the distro exists (if specified)

## Spawning

```bash
maxima --very-quiet
```

The `--very-quiet` flag suppresses the startup banner and version info.

**Binary detection order** (Local backend):
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
4. **Plot detection**: After parsing, scan `text_output` for SVG file path patterns matching `["/path/to/file.svg"]`. Extract the first `.svg` path, read the file, populate `plot_svg`, and strip the path line from `text_output`. Also suppress any LaTeX that just wraps the file path.
5. **Remaining lines**: plain text result

## Parsed Result Structure

```rust
pub struct EvalResult {
    pub cell_id: String,
    pub text_output: String,            // Filtered text lines
    pub latex: Option<String>,          // LaTeX without $$ delimiters
    pub plot_svg: Option<String>,       // SVG file content if plot was produced
    pub error: Option<String>,          // Raw error message if detected
    pub error_info: Option<ErrorInfo>,  // Structured error (title, explanation, did-you-mean)
    pub is_error: bool,
    pub duration_ms: u64,
    pub output_label: Option<String>,   // Maxima output label (e.g. "%o6")
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
  1. Send an interrupt signal to the Maxima process (SIGINT on Unix, CTRL_BREAK_EVENT on Windows)
  2. Drain output until the sentinel arrives (5 second timeout for the drain itself)
  3. Return an error result: "Evaluation timed out after 30 seconds"
  4. The session remains synchronized and ready for the next evaluation
- If the drain also times out, the process is truly stuck and the user should restart the session

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

When a computation exceeds the timeout, Aximar interrupts and resynchronizes:

1. Send SIGINT to the Maxima process (Unix) or CTRL_BREAK_EVENT (Windows, requires `CREATE_NEW_PROCESS_GROUP` at spawn)
2. Drain stdout/stderr until the sentinel line arrives (5 second drain timeout)
3. Return a timeout error to the caller

This keeps the protocol synchronized so subsequent evaluations work correctly, avoiding the need to restart the session after every timeout.

If the drain itself times out (Maxima completely unresponsive to interrupt), the session is left in a desynchronized state and the user should restart it manually.

## LaTeX Preprocessing for KaTeX

Maxima's `tex()` output needs minor adjustments for KaTeX:

| Maxima output | KaTeX-compatible |
|---------------|-----------------|
| `\over` | Works in KaTeX (keep as-is) |
| `\it ` | Replace with `\mathit{` |
| `\pmatrix{...}` | Replace with `\begin{pmatrix}...\end{pmatrix}` |

Build up the preprocessing function as edge cases are discovered during testing.
