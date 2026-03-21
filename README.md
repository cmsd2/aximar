# Aximar

A modern, cross-platform desktop GUI for the [Maxima](https://maxima.sourceforge.io/) computer algebra system. Aximar provides a notebook-style interface with beautifully rendered math output via KaTeX.

Built with [Tauri v2](https://tauri.app/) (Rust backend) and React + TypeScript (frontend).

## Features

- **Notebook interface** — multiple cells, add/delete freely
- **LaTeX math rendering** — Maxima output rendered with KaTeX
- **Keyboard-driven** — Shift+Enter to evaluate and advance
- **Session management** — auto-start, restart, status indicator
- **Dark theme** — easy on the eyes for long sessions

## Prerequisites

### Maxima

Aximar requires a working [Maxima](https://maxima.sourceforge.io/) installation.

| Platform | Install |
|----------|---------|
| macOS (Homebrew) | `brew install maxima` |
| Ubuntu/Debian | `sudo apt install maxima` |
| Fedora | `sudo dnf install maxima` |
| Windows | Download from [sourceforge.net/projects/maxima](https://sourceforge.net/projects/maxima/) |

Aximar looks for the `maxima` binary in these locations (in order):

1. `AXIMAR_MAXIMA_PATH` environment variable
2. `/opt/homebrew/bin/maxima`, `/usr/local/bin/maxima`, `/usr/bin/maxima`
3. Windows: `C:\maxima-*\bin\maxima.bat`
4. Falls back to `maxima` on `PATH`

### Build tools

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) (stable toolchain)
- Tauri v2 system dependencies — see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)

## Usage

### Running from source

```bash
npm install
npm run tauri dev
```

This starts both the Vite dev server (frontend hot-reload) and the Tauri Rust backend.

### Using the app

- **Type a Maxima expression** in a cell (e.g. `integrate(x^2, x);`)
- **Shift+Enter** — evaluate the cell and create a new cell below
- **Run button** (play icon) — evaluate a single cell
- **+ Cell** — add a new empty cell at the bottom
- **Run All** — evaluate all cells in order
- **Restart** — restart the Maxima session (clears Maxima's state, not your cells)
- **Delete** (x button, appears on hover) — remove a cell (minimum one cell always remains)

The session status indicator in the toolbar shows the current state: Ready, Starting, or Error.

### Example expressions

```
integrate(x^2, x);
diff(sin(x)*cos(x), x);
solve(x^2 - 5*x + 6 = 0, x);
expand((a + b)^4);
factor(x^4 - 1);
taylor(exp(x), x, 0, 5);
```

## Building

### Debug build

```bash
npm run tauri build -- --debug
```

### Release build

```bash
npm run tauri build
```

Build artifacts are placed in `src-tauri/target/release/bundle/`:
- macOS: `.app` bundle and `.dmg`
- Linux: `.AppImage` and `.deb`
- Windows: `.msi` installer

## Development

### Project structure

```
aximar/
├── src/                          # React frontend
│   ├── App.tsx                   # Root component, starts Maxima session
│   ├── main.tsx                  # Entry point
│   ├── components/
│   │   ├── Notebook.tsx          # Renders list of cells
│   │   ├── Cell.tsx              # Input textarea + output display
│   │   ├── CellOutput.tsx        # Dispatches to KaTeX or error view
│   │   ├── KatexOutput.tsx       # KaTeX rendering
│   │   ├── ErrorOutput.tsx       # Error display
│   │   └── Toolbar.tsx           # Add Cell, Run All, Restart, status
│   ├── hooks/
│   │   └── useMaxima.ts          # Cell execution and session logic
│   ├── lib/
│   │   ├── maxima-client.ts      # Tauri invoke wrappers
│   │   └── katex-helpers.ts      # LaTeX preprocessing for KaTeX
│   ├── store/
│   │   └── notebookStore.ts      # Zustand state (cells, session status)
│   ├── types/
│   │   ├── notebook.ts           # Cell, Notebook, CellOutput types
│   │   └── maxima.ts             # EvalResult, SessionStatus types
│   └── styles/
│       └── global.css            # Layout and dark theme
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # Entry point
│   │   ├── lib.rs                # Command registration and setup
│   │   ├── state.rs              # AppState (Maxima process handle)
│   │   ├── error.rs              # Error types
│   │   ├── maxima/
│   │   │   ├── process.rs        # Spawn/kill Maxima subprocess
│   │   │   ├── protocol.rs       # Sentinel-based send/receive
│   │   │   ├── parser.rs         # Parse LaTeX and errors from output
│   │   │   └── types.rs          # EvalResult, SessionStatus structs
│   │   └── commands/
│   │       ├── evaluate.rs       # evaluate_expression command
│   │       └── session.rs        # start/stop/restart session commands
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/
│   ├── implementation-plan.md    # Architecture and phased plan
│   └── maxima-protocol.md        # Maxima communication protocol details
├── package.json
├── vite.config.ts
└── tsconfig.json
```

### Architecture

The app communicates with Maxima through a long-lived subprocess. The Rust backend manages this process, sending expressions via stdin and reading results from stdout using a sentinel-based protocol. See `docs/maxima-protocol.md` for details.

```
React Frontend  ←→  Tauri IPC  ←→  Rust Backend  ←→  Maxima subprocess
                                                       (stdin/stdout)
```

### Key tech

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri v2 |
| Frontend | React 19, TypeScript, Vite |
| Math rendering | KaTeX |
| State management | Zustand |
| Subprocess I/O | tokio::process |

### Running tests

```bash
# Rust unit tests
cd src-tauri && cargo test

# TypeScript type checking
npx tsc --noEmit
```

### Maxima protocol

Each cell evaluation sends the user's expression plus `tex(%);` to get LaTeX output, followed by a sentinel string. The backend reads stdout until the sentinel appears, then parses the output to extract LaTeX, detect errors, and filter noise. Full protocol documentation is in `docs/maxima-protocol.md`.

## License

MIT
