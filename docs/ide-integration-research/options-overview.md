# Options Overview

## Current Strengths

**Aximar desktop app**: Rich notebook experience — KaTeX rendering, Plotly
plots, variable inspector, command palette, undo/redo, templates,
multi-notebook tabs. Ideal for interactive exploration.

**VS Code extension**: Proper code editing — syntax highlighting, completions,
go-to-definition, find references, workspace symbols, diagnostics, debugging
with breakpoints/stepping. Ideal for writing `.mac` libraries.

**Shared infrastructure**: Both consume `aximar-core` (catalog, docs, session
management) and `maxima-mac-parser`. MCP server supports HTTP transport with
shared state.

## Option 1: MCP Bridge (lowest effort, high value)

Keep both apps. Add commands to the VS Code extension that send code to a
running Aximar instance via MCP.

**What to build:**

- "Send Selection to Aximar" command — select code in VS Code, send to
  Aximar via MCP (`add_cell` + `run_cell`). See rendered LaTeX and plots in
  the Aximar window.
- "Open in Aximar" command — load a `.mac` file into Aximar as a notebook
  for interactive testing.
- Already documented in `docs/vscode-integration-usecases.md` (use cases 5.1,
  5.3) but not yet implemented.

**Pros:**

- Lowest effort — infrastructure is ~80% built
- Each tool stays focused on what it does best
- No compromises on either experience

**Cons:**

- Two separate windows
- Context switching between apps

**Effort:** 1-2 weeks.

## Option 2: VS Code Notebook API

Implement a VS Code notebook controller + serializer + custom renderers so
`.ipynb` Maxima notebooks can be opened and run inside VS Code.

See [vscode-notebook-api.md](vscode-notebook-api.md) for the full deep dive.

**Pros:**

- Single window with full VS Code editing power applied to notebook cells
- LSP works in cells (completions, diagnostics, hover)
- Custom renderers can do KaTeX + Plotly at the same quality
- `.ipynb` format compatibility with Aximar

**Cons:**

- Significant implementation effort
- Fixed single-column layout — cannot match Aximar's custom UI
- Feels like "a VS Code notebook," not like Aximar
- No notebook-level undo across cells
- No custom cell types (only Code and Markup)

**Effort:** 6-10 weeks for a polished implementation.

## Option 3a: Monaco Editor in Tauri

Embed Monaco Editor in Aximar to add code editing capabilities alongside the
notebook.

See [monaco-in-tauri.md](monaco-in-tauri.md) for the full analysis.

**Pros:**

- Single app, cohesive UX
- Full LSP via `monaco-languageclient`
- You control every pixel

**Cons:**

- You must build file explorer, git panel, debug UI, terminal, search —
  all inferior to VS Code's equivalents
- Massive ongoing maintenance burden
- ~14-24 weeks to reach "usable" quality

**Effort:** 14-24 weeks for usable; 6+ months for polished.

## Option 3b: Eclipse Theia

Replace Tauri with Theia (Electron-based IDE framework) to get professional
IDE features for free, then add the notebook as a custom Theia widget.

See [theia.md](theia.md) for the full analysis.

**Pros:**

- File explorer, git, terminal, debug UI, search — all production-quality,
  for free
- VS Code extension compatibility (existing extension works as-is)
- Proven in production (Arduino IDE, TI Code Composer, etc.)
- Deep customization via InversifyJS dependency injection

**Cons:**

- Requires switching from Tauri to Electron (~400-600 MB bundle)
- Notebook must be reimplemented as a Theia widget
- Rust backend becomes a sidecar process
- InversifyJS learning curve
- Monthly Theia releases to track

**Effort:** 12-20 weeks.

## Option 4: Hybrid (MCP Bridge + VS Code Notebook)

Build option 1 first (quick win), then add option 2 as a follow-up. VS Code
notebook evaluates cells via MCP to Aximar's running session.

**Pros:**

- Incremental investment
- MCP bridge is useful regardless
- VS Code notebook reuses the same evaluation backend

**Cons:**

- Still limited by VS Code notebook constraints
- Two implementations of notebook UX to maintain

**Effort:** 1-2 weeks (bridge) + 6-10 weeks (notebook).

## Recommendation

Start with **Option 1** (MCP bridge). It delivers the highest value per unit
of effort and the infrastructure is largely built. If the two-window workflow
proves insufficient, pursue **Option 2** (VS Code notebook) as a follow-up —
it reuses the same MCP evaluation backend and doesn't require abandoning
Aximar.

Avoid **Option 3a** (Monaco DIY) — you'd spend months building inferior
versions of features VS Code already does perfectly.

**Option 3b** (Theia) is viable if you decide the notebook must live inside a
full IDE, but it requires abandoning Tauri and the small-binary story, which
is a significant architectural pivot.
