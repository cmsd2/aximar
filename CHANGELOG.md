# Changelog

All notable changes to the standalone Maxima language tools are documented here.

This changelog covers releases tagged `tools-v*`. For the Aximar desktop app,
see the `v*` releases on GitHub.

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

[0.2.1]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.2.1
[0.2.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.2.0
[0.1.0]: https://github.com/cmsd2/aximar/releases/tag/tools-v0.1.0
