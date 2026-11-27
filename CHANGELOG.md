# Changelog

All notable changes to the standalone Maxima language tools are documented here.

This changelog covers releases tagged `tools-v*`. For the Aximar desktop app,
see the `v*` releases on GitHub.

## [Unreleased]

### aximar-mcp and maxima-dap

- **Errors are recognised under any locale**: Maxima translates its error
  messages, and the tools read them to tell when an evaluation failed. With
  the system language set to Spanish, a failed evaluation came back as a
  result (the one before it), and the debugger hung on a syntax error. The
  tools now start Maxima with `LANGUAGE=en`, which Maxima reads before
  `LC_ALL`, `LC_MESSAGES` and `LANG`.

- **Questions stop the cell instead of being answered**: when Maxima asked
  a question ("Is a positive, negative or zero?"), the tools answered it
  with a guess. A yes/no question hung the cell until the timeout, and the
  spare answers ran as input in the next cell. Now every question is a
  Maxima error, "Maxima needs to know: Is a positive, negative or zero?",
  and `error_info` suggests the line that answers it, such as
  `assume(a > 0)$`. `run_cell` returns it as a failed cell (`is_error`)
  rather than a tool error. As in Maxima, `errcatch` doesn't stop a
  question: `r: errcatch(integrate(exp(-a*x), x, 0, inf))` stops with the
  question, where it used to get the result for a guessed `positive`. Nor
  do library functions that catch errors internally, so
  `laplace(sin(a*t)/t, t, s)` asks for the sign of `a` instead of falling
  back to another method.

### aximar-mcp

- **Error explanations have IDs**: `error_info` gains `id` (such as
  `error.division_by_zero`), `params` with the values in its text, and
  `messages` with each field's catalog context and English template.
  `title`, `explanation` and `suggestion` keep their English text.
  [docs/maxima-messages.md](docs/maxima-messages.md) lists every ID.
- **Error text is Maxima's message alone**: the line " -- an error. To
  debug this try: debugmode(true);" no longer follows every error in
  `error`.
- **Lisp errors keep their cause**: SBCL prints a Lisp error's condition
  over several lines with blank lines between, and the tools kept only the
  first line. `error` now holds the whole condition on one line, without
  SBCL's "Automatically continuing." lines, and `error_info` explains a
  value of the wrong type (`error.lisp_type`), runaway recursion
  (`error.recursion`) and floating-point overflow (`error.float_overflow`).
  Runaway recursion is also an error on the kernel-events path, where it
  used to pass as print text.
- **Timeouts and cancellations leave an output**: a cell that timed out
  or was cancelled now has an error output saying so (`error.timeout`,
  `error.cancelled`), which `get_cell` returns, as well as the tool error.
- **Notes on Maxima's output**: a cell's output gains `notes`, each with
  an ID, English text and the range of text output lines it explains,
  for lines such as "rat: replaced 0.5 by 1/2 = 0.5" (one note for a run
  of them), "solve: using arc-trig functions to get a solution.",
  "Principal Value" and "expt: undefined: 0 to a negative exponent." in a
  cell that still has a result. Notes aren't written to files; opening a
  notebook finds them again.
- **More errors explained**: `log(0)`, an index past the end of a list or
  expression, taking parts of an atom, assigning to something that isn't
  a variable, `first` or `last` of an empty list, a non-polynomial given
  to `allroots`, and a system `algsys` can't reduce.
- **Syntax errors are explained by kind**: a missing `)` or `]`, input that
  ends too early, and two values with no operator between them each get
  their own explanation, naming the delimiter or the value. Input that ends
  too early used to get the generic syntax-error text.
- **Wrong argument counts name the definition**: calling a user function or
  lambda with the wrong arguments explains it with the definition and the
  arguments given ("f is defined as f(x), but was called with [1,2]."),
  and no longer suggests `describe`, which only knows Maxima's own
  functions.

## [0.6.2] — 2026-11-25

### maxima-dap

- **Step Out works**: VS Code waited forever after Step Out (Shift+F11),
  because maxima-dap acknowledged the request and did nothing. Maxima's
  debugger has no step-out command, so maxima-dap now steps until the run is
  back in a caller, stopping at the caller's next statement, at a breakpoint
  on the way, or at the end of the program. Recursive functions are handled
  by stack depth. `examples/17_step_out.mac` shows it.
- **Breakpoints no longer sit on lines Maxima can't stop on**: with stock
  Maxima, a breakpoint on a line that starts with arithmetic (`w * 2`, `v^2`)
  or holds only a value showed as set but never fired, because Maxima's
  evaluator skips its debugger hook for those. maxima-dap now moves such a
  breakpoint to the function's next line that can stop, or marks it
  unverified, with the reason, when there's none.
  `examples/18_lines_maxima_skips.mac` shows it.

## [0.6.1] — 2026-11-25

### aximar-mcp

- **Results come with their one-line form**: `evaluate_expression` and
  `run_cell` return `text_plain`, the final value as Maxima's `string()`
  writes it (`x^2/2`). The VS Code extension saves it as the result's
  `text/plain` in notebook files, which Jupyter shows.

### Notebook files

- **A result's LaTeX is no longer kept as its one-line form**: the VS Code
  extension up to v0.3.0 saved a copy of each result's LaTeX as its
  `text/plain`. Reading such a file took the copy for Maxima's one-line form
  and saved it again, so Jupyter showed LaTeX source as the result. A
  `text/plain` the same as the `text/latex` is now ignored. This applies to
  aximar-mcp, the desktop app and the `aximar-notebook-format` package.

## [0.6.0] — 2026-11-23

### aximar-mcp

- **`cargo install --git` builds again**: without `--locked`, cargo picked
  rmcp 1.3 or later, where `StreamableHttpServerConfig` can no longer be
  built with struct syntax, and the build failed. aximar-mcp now uses
  rmcp 1.8 and its builder methods. With 1.8, a server listening on loopback
  (the default) answers only requests whose `Host` is `localhost`,
  `127.0.0.1` or `::1`, which stops DNS rebinding; a server given another
  `--address` accepts any `Host` and relies on its bearer token, as before.
- **`aximar-mcp run` runs every cell**: batch runs no longer apply the
  dangerous-call check. Running the command is the decision to run the
  notebook, as with `maxima -b` or `jupyter nbconvert --execute`, and the
  check only matches names in the cell text, so it stopped ordinary
  notebooks (any `load` of a file it didn't know) without stopping one
  written to get past it. `run --allow-dangerous` is still accepted, with a
  warning that it is no longer needed. The desktop app, connected MCP and the
  standalone MCP server keep the check.
- **Every Maxima share package loads without trust**: the trust check only
  knew packages with `.mac` functions, under their folder names, so
  `load("orthopoly")`, `load("draw")` and other packages written in Lisp, and
  the usual `load("ctensor")` (rather than `load("tensor/ctensor")`), counted
  as loading an unknown file. That stopped `aximar-mcp run` without
  `--allow-dangerous` and showed the desktop app's trust banner. catalog-gen's
  new `share-files` command lists every `.mac` and `.lisp` file in Maxima's
  share directory (test scripts excluded), and a load that names one, by
  file name or path, passes the check.
- **Packages installed with mxpm load without `--allow-dangerous`**: the
  trust check treated `load("numerics")`, `load("ax-plots")` and other mxpm
  packages as loading an unknown file, so `aximar-mcp run` stopped at the
  first cell of any notebook that used one unless the whole check was off.
  A plain name that is an mxpm package in the Maxima user directory, or a
  `.mac` or `.lisp` file at the top of one (`load("numerics-optimize")`), now
  counts as a known package, like Maxima's own share packages. The desktop
  app's trust banner no longer appears for these loads either. Paths and
  other files still need trust.
- **Catalog tools cover functions that only a package provides**:
  `search_functions` lists functions it finds by name after the catalog's
  ranked results, without a score, marking those with no documentation
  `undocumented: true`; `get_function_docs` answers for them with every
  package that provides them and how to load it, instead of "not found";
  completion names every such package; and a misspelt name suggests package
  functions as well as documented ones. The desktop app and MCP now
  share one search, documentation and completion API in `aximar-core`.
- **`run_all_cells` reports cells that did not succeed**: each result now has a
  `status` (`ok`, `empty`, `error`, `needs_trust`, `failed`, `not_run`). A cell
  held back because the notebook is untrusted used to count as `success: true`,
  and so did a cell whose evaluation ended in a Maxima error. The tool now stops
  at the first such cell, as its description said, marks the remaining cells
  `not_run`, and adds top-level `success`, `stopped_at`, `stop_reason` and
  `message` fields. The unused `pending_approval` field is gone.
- **Notebooks keep the file they were opened from or saved to** (#40):
  `open_notebook` and `save_notebook` record the path, so `list_notebooks`
  reports it with the file name as the title, and Maxima runs in the file's
  folder from the session's next start. `load_template` clears it. In
  connected mode the window's tab follows: it shows the file name and saves
  to that file.
- **`evaluate_expression` understands cell labels in notebook mode**: `%o4`
  and `%i4` in an expression refer to the notebook cells' `Out[4]` and `In[4]`,
  as they do in cells, and a label with no value in the session is reported
  instead of reading an unrelated Maxima label. A bare `%` still means
  Maxima's last result. Simple mode (sessions, no notebook) keeps Maxima's own
  labels unless the call names a client cell, so clients that resolve labels
  themselves keep working.
- **Simple mode runs a client's notebook cells by ID** (#8): `evaluate_expression`
  takes optional `cell_id` and `cells_above` (the IDs of the code cells above,
  in order), so a client that keeps its own notebook, such as the VS Code
  extension, can send cell source unchanged. The server numbers the run,
  returns it as `execution_count`, and resolves `%oN` and a bare `%` by the
  desktop app's rules: a cell that failed, printed nothing, or hasn't run in
  this session is never the output above.
- **An opened notebook numbers its runs from 1** (#8): `open_notebook` and
  `load_template` start the gutter numbers again at `In[1]`, whatever the
  file's outputs show, as a new session does. Those outputs still have no
  value as labels until their cells run.
- **`run_all_cells` shares the desktop app's Run All** (#41): both run in
  `aximar-core` and stop at the same cells.
  - Stopping a run from the desktop window ends it after the current cell,
    with `stop_reason` `cancelled`.
  - Standalone, a cell blocked by the trust rule is reported as `needs_trust`
    (with `dangerous_functions`) instead of `failed`.
  - Batch runs (`aximar-mcp run`) use the same code; output and exit codes are
    unchanged.
- **One trust rule for cells and expressions** (#39):
  - **Breaking, connected mode:** `evaluate_expression` follows the notebook's
    trust flag, like `run_cell`. It used to block dangerous calls even in a
    trusted notebook.
  - In connected mode, a cell blocked because the notebook is untrusted now
    shows the trust banner in the desktop window, so the user can trust the
    notebook there. Before, only the MCP client heard about it.
  - Blocked-call messages say what would allow the call: trusting the notebook
    in connected mode, `--allow-dangerous` when standalone.
- **Connected mode starts Maxima with the app's settings**: a session that an
  MCP client started in the desktop app used the local Maxima on the PATH and
  a 30-second timeout, whatever the app's settings said. It now uses the
  backend, Maxima path and timeout from Settings, read when the session starts,
  so WSL, Docker and custom-path setups work over MCP.
- **`list_cells` no longer fails on multi-byte output**: the 200-character
  preview was cut at a byte offset, which panicked inside a Greek letter or
  other non-ASCII character.

### maxima-dap and maxima-lsp

- **The debugger opens package files outside the program's folder with stock
  Maxima** (#43): stepping into a function from a package found on
  `file_search_maxima` showed "the file was not found", because `:bt` gives
  only a file name. maxima-dap now takes each frame's full path from
  `:frame N`, as it already did with the enhanced debugger.
- **A program's top-level code runs before `evaluate` with stock Maxima**:
  maxima-dap's Legacy mode (Maxima without the enhanced debugger) loaded only
  the program's definitions and then ran `evaluate`, so top-level `load`s,
  settings and globals were skipped whenever `evaluate` was set. Calls into
  a package loaded that way did nothing, and Step Into ran to the end.
  Enhanced mode already ran the file first.
- **Comments outside functions no longer stop a launch with stock Maxima**:
  Legacy mode ran the top-level code with its comments, so a comment after
  the last top-level statement, or a file whose only top-level text was a
  comment, failed with `incorrect syntax: , is not a prefix operator`.
- **`--version` and `--help`**: both servers print and exit. Before, they
  started the server and waited for an editor on stdin, so
  `maxima-dap --version` hung. Run from a terminal, either server now says on
  stderr that an editor starts it. `--stdio` is accepted, for editors that
  pass it.

### aximar-core

- **Unicode in Maxima code follows a defined table**, set out in
  `docs/unicode-input.md`. Each character becomes a Maxima name, constant or
  operator, passes through, or is rejected before Maxima runs with a message
  that says what to type instead.
  - New: variant letters (`ϕ`, `ϑ`, `ϵ`, `µ`, `∆`, mathematical italic) are
    their letter; `Γ(5)` calls the gamma function; `√2`, `√(x+1)`; `−`, `⋅`,
    `∗`, `∧`, `∨`, `¬`; `ⅈ`, `ⅇ`, `ℯ`, `ℏ`, `ℓ`; subscript letters (`aₙ₊₁`);
    pasted no-break spaces, zero-width characters, full-width forms and
    ligatures are cleaned up.
  - `ε` and `φ` display as typed instead of as `ϵ` and `ϕ`.
  - **Breaking:** a Greek letter inside a longer name stays as typed (`Δx`
    was `Deltax`, `xπ` was `x%pi`); variant letters are no longer separate
    names; Greek capitals that look like Latin letters, and omicron, are
    rejected; comments, and strings containing `\"`, are no longer
    translated; `Γ(...)` is the gamma function rather than a function named
    `Gamma`.
- **Label references resolve only to values from the current session**:
  - After a Maxima restart, execution numbers start again at 1, as in
    Jupyter. Restarting and running all cells gives them the same `In[n]` and
    `Out[n]` as before, so references such as `%o4` keep working.
  - A `%oN` or `%iN` with no value in the current session (a cell that
    errored, a cell loaded from a file, a number from before a restart) used
    to reach one of Maxima's own labels and return an unrelated value. The
    cell now shows an error such as "Out[6] (%o6) has no value in this
    session".
  - Bare `%` refers to the previous cell only in a cell's first statement;
    in `x: 5; % + 1;` it is now the result of `x: 5`, as in Maxima.
  - `%` and labels inside strings and comments are left as typed.
- **Output of a running cell is kept**: a run queued behind it no longer
  clears its captured output.
- **Docker sessions get unique container names**: a second notebook on the
  Docker backend failed to start, and stopping one could remove another's
  container.
- **Catalog search can't panic on a NaN score**.

## [0.5.0] — 2026-11-03

maxima-lsp and maxima-dap have no changes of their own; their versions move to
0.5.0 with aximar-mcp.

### aximar-core

- **Bundled ax-plots re-synced to 0.2.0**, plus the 0.2.1 grid fix below.
  Adds labels, arrows, polygons and shapes; arrowheads and snake/zigzag
  decorations on curves; per-object options; `theme="dark"` and the diagram preset;
  `height=fit`; `group()` and lists of objects; `animate()`; and reactive
  plot metadata for 2D and 3D plots. See the
  [ax-plots changelog](https://github.com/cmsd2/ax-plots/blob/master/CHANGELOG.md).
- **Fixed transposed 2D grid plots**: `implicit` (2D), `explicit` (3D
  surfaces), `ax_contour`, `ax_contour3d` and the expression form of
  `ax_heatmap` drew f(y, x) instead of f(x, y).
- **Fixed plots that needed the numerics package**: `ax_draw3d(points(...))`,
  `ax_heatmap` on a matrix and `ax_bar` with a matrix of columns failed with
  `first: argument must be a non-atomic expression` or
  `map: improper argument: true` unless numerics was loaded.
- **Results of 0 display again**: a cell that evaluated to 0 rendered empty.
- **Wrapped LaTeX keeps its spaces**: when `tex()` wrapped a long result, a
  control word could fuse with the next token (`\log a` became `\loga`) and
  show as raw text.
- **Fixed output leaking between commands**: each protocol sentinel also
  printed its return value as a stray line, which could put the variables list
  into a cell's text output or cut off a cell's last statement.
- **Solve and right-hand-side suggestions appear for equations**: equation
  detection never matched anything, so these suggestions never appeared.
  `:=` and `::=` definitions don't count as equations.
- **Session status after a timeout kill**: when an evaluation timed out and
  Maxima was killed, the session kept reporting Ready and later commands
  failed with "Broken pipe". It now moves to an error state that points at
  `restart_session`.
- **Experimental kernel-events protocol**, off by default. On Unix,
  `AXIMAR_KERNEL_EVENTS=1` gives Maxima an event channel on fd 3 and a cancel
  channel on fd 4, and results, errors, variables and plots arrive as JSON
  envelopes instead of scraped stdout. It needs the `kernel-events` Maxima
  package and an SBCL-based Maxima; without the package, evaluations run to
  the timeout.

### aximar-mcp

- Inherits all aximar-core changes above.

## [0.4.1] — 2026-04-22

### aximar-mcp

- **Fixed missing output fields**: `image_png` and `plot_data` fields
  are now included in `evaluate_expression` and cell suggestion
  tool responses.

## [0.4.0] — 2026-04-22

### aximar-core

- **Save/load cell outputs**: Notebook files now persist text, LaTeX, plot, and
  image outputs so they render when reopened without re-running cells.
- **Plot persistence**: Plotly JSON (`application/x-maxima-plotly`) and SVG
  (`image/svg+xml`) outputs are saved/loaded via nbformat `display_data` entries.
- **Image output support**: Base64-encoded `image/png` and `image/jpeg` outputs
  are saved and rendered in notebooks.
- **Notebook-level trust**: Dangerous function approval is now per-notebook
  instead of per-cell, unblocking run-all-cells workflows.
- **Trailing comment stripping**: Block comments after the last statement
  terminator (e.g. `x:1; /* note */`) are stripped before sending to Maxima,
  preventing parse errors.
- **Comment-only cell handling**: Cells containing only comments are detected
  as empty after stripping and skipped gracefully.
- **Assumption question detection**: Maxima assumption prompts (e.g. "Is x
  positive?") are now detected and returned as errors instead of hanging.
- **SBCL compiler warning filtering**: Spurious SBCL compiler notes in Maxima
  output are now stripped from results.

### aximar-mcp

- **CLI batch runner**: `aximar-mcp run <path> [-o output] [--allow-dangerous]`
  executes all cells in a notebook and saves with outputs, enabling scripted
  notebook execution for CI and batch processing.
- **Clap CLI**: Replaced manual argument parsing with clap derive macros.
  Adds `--help`, `--version`, and subcommand help.
- Inherits all aximar-core changes above.

## [0.3.1] — 2026-04-17

### aximar-core

- **Fixed doc-index loading**: When both a full and slim doc-index exist for
  the same package (e.g. `maxima-core-docs`), the full version is now preferred.
  Previously, non-deterministic directory iteration order could cause the slim
  variant to overwrite the full docs.

## [0.3.0] — 2026-04-17

### maxima-lsp

- **Doc-index support**: Load `doc-index.json` from installed packages
  (`~/.maxima/*/doc/`) for hover docs, completions, and signature help.
- **Auto-reload**: File watcher detects when packages are installed or removed
  and reloads doc-index data automatically.

### aximar-core

- **Fixed polar plot JSON**: `simplode([], ",")` returns `false` in Maxima;
  empty `polar_parts` now correctly produces `"polar":{}`.
- **Added `save_plotly_json` suggestion**: Plotly plot outputs now offer a
  "Save JSON" suggestion chip.

### aximar-mcp

- Inherits aximar-core fixes above.

## [0.2.2] — 2026-04-14

### aximar-core

- **3D parametric curves**: `parametric(x(t), y(t), z(t), t, t0, t1)` in `ax_draw3d`
  renders 3D space curves (helices, knots, etc.) as `scatter3d` traces.
- **3D parametric surfaces**: `parametric_surface(x(u,v), y(u,v), z(u,v), u, u0, u1, v, v0, v1)`
  in `ax_draw3d` renders surfaces like spheres, tori, and Möbius strips.
- **3D implicit surfaces**: `implicit(eqn, x, x0, x1, y, y0, y1, z, z0, z1)` in
  `ax_draw3d` renders isosurfaces using Plotly's `isosurface` trace.
- **3D surface contours**: `ax_contour3d(expr, x, x0, x1, y, y0, y1)` renders a
  surface with contour lines projected below.
- **3D vector fields**: `ax_vector_field3d(Fx, Fy, Fz, x, x0, x1, y, y0, y1, z, z0, z1)`
  renders 3D vector fields as cone plots.
- **Box plots**: `ax_box(data)` or `ax_box(data, category)` — box-and-whisker plots
  with `boxpoints` and `boxmean` options.
- **Violin plots**: `ax_violin(data)` or `ax_violin(data, category)` — kernel density
  visualization with embedded box plot and mean line.
- **Pie/donut charts**: `ax_pie(values, labels)` with `hole` option for donut charts.
- **Error bars**: `ax_error_bar(xs, ys, y_errors)` or
  `ax_error_bar(xs, ys, y_errors, x_errors)` — scatter plots with error bars.
- **New template notebooks**: "3D Plotting" and "Statistics" templates with worked
  examples for all new chart types.
- **Fixed `parametric_surface` argument count**: Was incorrectly requiring 10
  arguments instead of 9.
- **Fixed implicit 3D and vector field 3D performance**: Replaced O(n²) `endcons`
  list-building in triple-nested loops with pre-allocated arrays and indexed
  assignment. Reduced default implicit grid from 30³ to 20³.

### aximar-mcp

- **Float gotcha in AI instructions**: Added guidance about wrapping expressions
  containing symbolic constants (`%pi`, `%e`) in `float()` to force numeric
  evaluation — prevents severe performance issues with large symbolic lists.

## [0.2.1] — 2026-04-13

### maxima-dap

- **Runtime errors surfaced in debug GUI**: When Maxima hits a runtime error
  (e.g. `ev: improper argument`), the stopped event now reports
  `reason: "exception"` with the error message, instead of a generic breakpoint
  stop with no context.
- **Synthetic stack frame for top-level errors**: When the backtrace is empty
  (error outside any user-defined function), a synthetic frame is generated from
  the canonical location so the call stack panel isn't blank.
- **Fixed canonical location parsing**: The regex now handles the `\x1a\x1a`
  Emacs/GDB annotation prefix that Maxima emits, which was silently preventing
  all canonical location matching.
- **Fixed breakpoint deletion**: Use `:delete` (the correct Maxima command)
  instead of `:delbreak` (nonexistent).

### aximar-mcp

- **Comma/ev gotcha in instructions**: Added warning that the comma operator
  in Maxima is `ev()`, not a statement separator.

## [0.2.0] — 2026-04-13

### maxima-dap

- **Improved breakpoint resolution**: Breakpoint locations are now captured
  directly from execution output (e.g. `batchload`, `:resume`, `:step`) instead
  of querying `:info :bkpt` after each stop. This gives exact full-path file
  matching and eliminates a round-trip to the Maxima process on every debug stop.

## [0.1.0] — 2026-04-13

Initial standalone release of the Maxima language tools.

### maxima-lsp

- Syntax-aware completions for 2500+ built-in Maxima functions
- Hover documentation from the Maxima reference manual
- Go-to-definition and references for user-defined functions
- Real-time diagnostics (syntax errors, unmatched parens)
- Document and workspace symbols

### maxima-dap

- Step-through debugging with breakpoints, variable inspection, and call stacks
- Enhanced Maxima debugger support (file:line breakpoints with deferred resolution)
- Legacy mode fallback for stock Maxima (function+offset breakpoints)
- Output filtering to suppress debugger noise
- Canonical file path handling for reliable source mapping
- Configurable evaluation timeouts

### aximar-mcp

- HTTP transport with bearer token authentication
- Multi-notebook session management (create, close, restart)
- Cell evaluation with LaTeX, SVG, and Plotly output
- Documentation catalog with full-text search
- Dangerous function safety gates
- Structured JSON startup output for VS Code integration

[0.6.2]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.6.2
[0.6.1]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.6.1
[0.6.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.6.0
[0.5.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.5.0
[0.4.1]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.4.1
[0.4.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.4.0
[0.3.1]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.3.1
[0.3.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.3.0
[0.2.2]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.2.2
[0.2.1]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.2.1
[0.2.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.2.0
[0.1.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.1.0
