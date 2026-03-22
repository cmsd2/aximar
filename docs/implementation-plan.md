# Aximar — Implementation Plan

A modern, cross-platform desktop GUI for the Maxima computer algebra system.

## Overview

Aximar provides a notebook-style interface (like Jupyter/Mathematica) for Maxima, with beautifully rendered math output via KaTeX, syntax-highlighted editing via CodeMirror, and inline plot display. It is built with Tauri v2 (Rust backend) and Vite + React + TypeScript (frontend).

## Tech Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Desktop shell | Tauri v2 (Rust) | Small binary (~5-10MB), native feel, secure |
| Frontend | Vite + React 19 + TypeScript | Fast dev, large ecosystem, Tauri default template |
| Math rendering | KaTeX | Fast LaTeX rendering, same quality as modern LaTeX |
| Code editor | Plain `<textarea>` (CodeMirror 6 planned) | Lightweight, with custom autocomplete and hover tooltips |
| State management | Zustand | Minimal API, no boilerplate, fine-grained reactivity |
| Styling | CSS Modules + CSS custom properties | Scoped styles, no build complexity |
| Subprocess mgmt | tokio::process | Full async control over Maxima stdin/stdout |

## Architecture

```
┌──────────────────────────────────────────────┐
│                 Tauri Window                  │
│  ┌────────────────────────────────────────┐  │
│  │            React Frontend              │  │
│  │  ┌──────────┐  ┌───────────────────┐  │  │
│  │  │ Toolbar  │  │    StatusBar      │  │  │
│  │  └──────────┘  └───────────────────┘  │  │
│  │  ┌──────────────────────────────────┐  │  │
│  │  │  Notebook                        │  │  │
│  │  │  ┌────────────────────────────┐  │  │  │
│  │  │  │ Cell                       │  │  │  │
│  │  │  │  CellInput (CodeMirror)    │  │  │  │
│  │  │  │  CellOutput (KaTeX/Plot)   │  │  │  │
│  │  │  └────────────────────────────┘  │  │  │
│  │  │  ┌────────────────────────────┐  │  │  │
│  │  │  │ Cell ...                   │  │  │  │
│  │  │  └────────────────────────────┘  │  │  │
│  │  └──────────────────────────────────┘  │  │
│  └────────────────────────────────────────┘  │
│         │ Tauri invoke (IPC)                 │
│  ┌──────┴─────────────────────────────────┐  │
│  │          Rust Backend                   │  │
│  │  ┌──────────────────────────────────┐  │  │
│  │  │  Commands (evaluate, session)    │  │  │
│  │  └──────────┬───────────────────────┘  │  │
│  │  ┌──────────┴───────────────────────┐  │  │
│  │  │  Maxima Module                   │  │  │
│  │  │   process.rs  — spawn/kill       │  │  │
│  │  │   protocol.rs — sentinel I/O     │  │  │
│  │  │   parser.rs   — LaTeX/errors     │  │  │
│  │  └──────────┬───────────────────────┘  │  │
│  └─────────────┼───────────────────────────┘  │
└────────────────┼──────────────────────────────┘
                 │ stdin/stdout pipes
          ┌──────┴──────┐
          │   Maxima    │
          │  subprocess │
          └─────────────┘
```

## File Structure

```
aximar/
├── docs/
├── package.json
├── vite.config.ts
├── index.html
├── src/                              # Frontend
│   ├── main.tsx
│   ├── App.tsx                      # Top-level layout, menu event listener, file ops
│   ├── types/
│   │   ├── notebook.ts              # Cell, CellOutput, Notebook types
│   │   ├── notebooks.ts             # Jupyter nbformat types (NotebookCell, etc.)
│   │   ├── maxima.ts                # EvalResult, SessionStatus, ErrorInfo
│   │   └── suggestions.ts           # Suggestion type (with optional action)
│   ├── store/
│   │   ├── notebookStore.ts         # Zustand: cells, filePath, isDirty, save/load
│   │   └── logStore.ts              # Log panel state
│   ├── components/
│   │   ├── Notebook.tsx
│   │   ├── Cell.tsx
│   │   ├── CellOutput.tsx           # Renders KaTeX, plot SVG, or text
│   │   ├── CellSuggestions.tsx      # Suggestion chips (eval + actions like Save SVG)
│   │   ├── KatexOutput.tsx
│   │   ├── EnhancedErrorOutput.tsx  # Rich error display with did-you-mean
│   │   ├── HoverTooltip.tsx         # Function hover docs
│   │   ├── Toolbar.tsx              # Toolbar with filename/dirty indicator
│   │   ├── CommandPalette.tsx       # Cmd+K function browser
│   │   ├── TemplateChooser.tsx      # Template selection modal
│   │   ├── SettingsModal.tsx
│   │   ├── VariablePanel.tsx
│   │   ├── DocsPanel.tsx
│   │   └── LogPanel.tsx
│   ├── hooks/
│   │   ├── useMaxima.ts             # Cell execution logic
│   │   ├── useAutocomplete.ts       # Autocomplete popup logic
│   │   ├── useHoverTooltip.ts       # Function hover detection
│   │   └── useTheme.ts
│   ├── lib/
│   │   ├── maxima-client.ts         # Tauri invoke wrappers
│   │   ├── notebooks-client.ts      # Save/open/template client (uses dialog plugin)
│   │   ├── catalog-client.ts        # Search/complete/get functions
│   │   ├── suggestions-client.ts
│   │   ├── config-client.ts
│   │   └── textarea-caret.ts
│   └── styles/
│       └── global.css
└── src-tauri/                        # Rust backend
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json     # core, opener, dialog permissions
    └── src/
        ├── main.rs
        ├── lib.rs                    # Plugin + command registration, setup
        ├── menu.rs                   # Native menu bar (File, Edit, Window)
        ├── state.rs                  # AppState (Maxima handle, catalog)
        ├── error.rs
        ├── maxima/
        │   ├── mod.rs
        │   ├── process.rs            # Spawn/kill/restart subprocess
        │   ├── protocol.rs           # Sentinel-based send/receive
        │   ├── parser.rs             # Parse LaTeX, errors, SVG plots
        │   ├── errors.rs             # Error pattern matching + enhancement
        │   └── types.rs              # EvalResult, SessionStatus, ErrorInfo
        ├── catalog/
        │   ├── search.rs             # Function catalog search + completion
        │   └── catalog.json          # Embedded function metadata
        ├── notebooks/
        │   ├── mod.rs
        │   ├── data.rs               # Embedded template loading
        │   ├── io.rs                 # Read/write notebook files
        │   ├── types.rs              # Notebook, NotebookCell (nbformat 4)
        │   ├── welcome.json
        │   ├── calculus.json
        │   ├── linear-algebra.json
        │   ├── equations.json
        │   ├── programming.json
        │   └── plotting.json         # 2D/3D/parametric plot examples
        ├── suggestions/
        │   ├── types.rs              # Suggestion (with optional action field)
        │   └── rules.rs              # Context-aware suggestion generation
        └── commands/
            ├── mod.rs
            ├── evaluate.rs
            ├── session.rs
            ├── config.rs
            ├── catalog.rs
            ├── suggestions.rs
            ├── notebooks.rs          # list/get templates, save/open notebooks
            ├── variables.rs
            └── plot.rs               # write_plot_svg command
```

---

## Maxima Communication Protocol

This is the most critical design element. Maxima runs as a long-lived subprocess.

### Initialization

Spawn Maxima with `--very-quiet` to suppress banners. Then send:

```
display2d:false$
set_plot_option([run_viewer, false])$
set_plot_option([gnuplot_term, svg])$
print("__AXIMAR_READY__")$
```

Wait for `__AXIMAR_READY__` on stdout to confirm the process is initialized and ready.

### Evaluation Protocol

For each cell execution, send this sequence to stdin:

```
set_plot_option([gnuplot_out_file, "<temp_dir>/<cell_id>.svg"])$
<user expression>
tex(%);
print("__AXIMAR_EVAL_END__");
```

Then read stdout line-by-line until `__AXIMAR_EVAL_END__` is detected.

### Output Parsing

The collected output lines are parsed into an `EvalResult`:

1. **LaTeX**: Lines matching `$$...$$` — strip delimiters, preprocess for KaTeX compatibility
2. **Errors**: Lines containing ` -- an error.` or `incorrect syntax:`
3. **Filter**: Remove `false` (tex return value), sentinel echo, `"__AXIMAR_EVAL_END__"`
4. **Plots**: Check if `<temp_dir>/<cell_id>.svg` exists → read content
5. **Remaining**: lines become the plain text result

### KaTeX Preprocessing

Maxima's `tex()` output needs preprocessing for KaTeX compatibility:

- `\over` works in KaTeX but should be converted to `\frac{}{}` for reliability
- `\it` → `\mathit{}`
- Strip `$$` delimiters (KaTeX handles display mode separately)

### Timeout & Error Recovery

- 30-second default timeout per evaluation
- On timeout: kill Maxima process, restart, notify frontend
- On crash: detect via `child.try_wait()`, auto-restart, notify frontend

---

## Implementation Phases

### Phase 1: MVP — Working Notebook with Math Rendering ✅

**Goal**: A usable app where you can type Maxima expressions, execute them, and see beautifully rendered math output with KaTeX.

**Backend (Rust):**
1. Scaffold project: `npm create tauri-app@latest` (react-ts template)
2. Add Rust deps to `Cargo.toml`: tokio (full), regex, thiserror, tempfile
3. Add npm deps: katex, zustand, nanoid
4. Implement Maxima subprocess management (`maxima/process.rs`)
5. Implement sentinel protocol (`maxima/protocol.rs`)
6. Implement output parser (`maxima/parser.rs`)
7. Implement Tauri commands (`commands/evaluate.rs`, `commands/session.rs`)
8. Register commands, configure window (1200x800, min 800x600)

**Frontend (React + TypeScript):**
9. Create TypeScript types (`types/notebook.ts`, `types/maxima.ts`)
10. Create Zustand store (`store/notebookStore.ts`)
11. Create Tauri invoke wrappers (`lib/maxima-client.ts`)
12. Build Notebook → Cell → Input/Output component hierarchy
13. Cell input uses plain `<textarea>` (CodeMirror deferred to Phase 2)
14. Cell output renders LaTeX via KaTeX (`KatexOutput.tsx`)
15. Error output component (`ErrorOutput.tsx`)
16. LaTeX preprocessing for KaTeX compatibility (`lib/katex-helpers.ts`)
17. Build Toolbar (Add Cell, Run All, Restart)
18. Shift+Enter to execute cells
19. Add/delete cell operations
20. Basic CSS layout and styling

**Verify**: `npm run tauri dev` → type `integrate(x^2, x);` → Shift+Enter → see rendered math.

### Phase 2: CodeMirror Editor

**Goal**: Syntax-highlighted Maxima editing replaces textarea. (Currently using plain `<textarea>` with autocomplete and hover tooltips.)

1. Create basic Maxima language mode for CodeMirror 6
   - Keywords: `if`, `then`, `else`, `do`, `for`, `while`, etc.
   - Built-ins: `integrate`, `diff`, `solve`, `expand`, `factor`, etc.
   - Numbers, strings, comments (`/* ... */`)
2. Build CellInput wrapping CodeMirror
3. Shift+Enter keybinding, line numbers, bracket matching
4. Auto-expanding editor height
5. Theme (One Dark or custom light theme)

**Verify**: Syntax coloring works, Shift+Enter executes, editor feels responsive.

### Phase 3: Plot Support ✅

**Goal**: `plot2d(sin(x), [x, -3, 3])` renders inline SVG.

1. ~~Set unique `gnuplot_out_file` per cell in protocol~~ (Maxima writes to its own temp file)
2. ✅ Parser detects SVG file path pattern in Maxima output via regex, reads SVG content, strips path from text output
3. ✅ `CellOutput` renders `plotSvg` inline via `dangerouslySetInnerHTML` (trusted local content)
4. ✅ `.plot-output` CSS: centered, responsive, light background
5. ✅ Plotting template with 2D, 3D, parametric, and Lissajous examples
6. ✅ "Save SVG" suggestion chip for plot outputs (opens native save dialog)

**Verify**: Plot commands produce visible inline graphs. ✅

### Phase 4: Polish + Persistence ✅

**Goal**: Production-quality UX with file save/load.

1. ✅ Cell reorder (move up/down)
2. ✅ In[N] / Out[N] execution labels (`output_label` and `executionCount`)
3. ✅ Status bar (session status)
4. ✅ Loading spinner during evaluation
5. ✅ Keyboard shortcuts:
   - ✅ `Shift+Enter` — run cell, advance to next
   - ✅ `Ctrl/Cmd+Enter` — run cell in place
   - ✅ `Escape` — blur editor
6. ✅ Run All
7. ✅ Responsive layout at various window sizes
8. ✅ `.axm` / `.ipynb` JSON notebook format (Jupyter nbformat 4)
9. ✅ Save/Load commands with native file picker dialog (`tauri-plugin-dialog`)
10. ✅ Native macOS menu bar with File menu (New, Open, Save, Save As) + accelerators
11. ✅ Unsaved changes warning on close
12. ✅ Dirty state tracking — toolbar shows filename and `*` indicator

### Phase 5: Cross-Platform Distribution (partially complete)

**Goal**: Distributable app for macOS, Linux, Windows.

1. ✅ Cross-platform Maxima binary detection (`AXIMAR_MAXIMA_PATH` env var + settings)
2. ✅ "Maxima not found" dialog with platform-specific install instructions
3. App icons (Tauri icon generator)
4. Build config for .dmg / .AppImage / .deb / .msi

### CI/CD ✅

**Goal**: Automated builds on every push to `master`.

A GitHub Actions workflow (`.github/workflows/build.yml`) builds the app across macOS, Linux, and Windows using a matrix strategy. Each job:

1. Checks out the repo
2. Installs Rust stable + cargo cache (`Swatinem/rust-cache`)
3. Installs system deps (Linux only: GTK3, WebKit2GTK 4.1, appindicator, librsvg, patchelf, OpenSSL)
4. Sets up Node 22 LTS with npm cache
5. Runs `npm ci` → `npm run build` (typecheck + Vite bundle) → `npx tauri build`
6. Uploads platform bundles as artifacts (.dmg/.app, .deb/.AppImage, .msi/.exe)

Maxima is **not** required at build time — it's a runtime-only dependency.

---

## Key Design Decisions

### Why tokio::process instead of Tauri Shell Plugin?

Maxima is a long-lived process with complex bidirectional I/O and sentinel-based protocol. `tokio::process` gives full control over stdin/stdout buffering, timeouts, and async reads. The Shell plugin is designed for simpler frontend-initiated commands.

### Why sentinel protocol instead of parsing output labels?

Maxima's `(%oN)` labels can be suppressed (with `$` terminator), and some expressions produce no labeled output. The sentinel `__AXIMAR_EVAL_END__` always fires regardless of what the user typed, giving reliable output boundary detection.

### Why SVG inline instead of asset protocol for plots?

SVG is text and serializes efficiently through Tauri's JSON IPC. No need for custom protocol handlers or filesystem access from the frontend.

### Why Zustand?

Minimal API, excellent TypeScript support, selector-based subscriptions prevent unnecessary re-renders. Our state shape (list of cells) doesn't warrant Redux complexity.

### Why CSS Modules instead of Tailwind?

Focused UI with few components. CSS Modules provide scoping without class-name verbosity. Keeps JSX clean.

---

## Dependencies

### Rust (Cargo.toml)

```toml
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
regex = "1"
thiserror = "2"
tempfile = "3"
```

### npm

```
@tauri-apps/api @tauri-apps/plugin-opener @tauri-apps/plugin-dialog
katex @types/katex
zustand nanoid
react-markdown rehype-katex remark-math
```

(React, TypeScript, Vite, @tauri-apps/cli come from the template.)
