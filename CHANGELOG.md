# Changelog

All notable changes to the standalone Maxima language tools are documented here.

This changelog covers releases tagged `tools-v*`. For the Aximar desktop app,
see the `v*` releases on GitHub.

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

[0.2.2]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.2.2
[0.2.1]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.2.1
[0.2.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.2.0
[0.1.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.1.0
