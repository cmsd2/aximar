# Aximar

A modern, cross-platform desktop GUI for the [Maxima](https://maxima.sourceforge.io/) computer algebra system. Aximar provides a notebook-style interface with beautifully rendered math output via KaTeX.

Built with [Tauri v2](https://tauri.app/) (Rust backend) and React + TypeScript (frontend).

![Aximar screenshot — 2D plot rendered inline](assets/screenshot-2d-plot.png)

## Features

- **Notebook interface** — code and markdown cells with drag-to-reorder
- **LaTeX math rendering** — Maxima output rendered with KaTeX
- **Inline plots** — 2D, 3D, and parametric plots displayed as SVGs
- **Function discovery** — searchable catalog of 2500+ functions, docs panel, hover tooltips
- **Command palette** — quick access to functions and categories (Ctrl+K / Cmd+K)
- **Smart editor** — syntax highlighting, autocomplete, signature hints
- **Find & replace** — search across cells with regex support
- **Error enhancement** — friendly explanations, "did you mean?" suggestions, correct signatures
- **Variables panel** — inspect and manage bound variables
- **Templates** — starter notebooks for calculus, linear algebra, plotting, and more
- **Dark/light/auto theme** — follows your system preference or set manually
- **Multiple backends** — run Maxima locally, in Docker/Podman, or via WSL
- **Save & load** — persist notebooks in Jupyter-compatible format
- **Print support** — configurable margins and font sizes for printing

## Getting Started

### Installing Maxima

Aximar requires a working [Maxima](https://maxima.sourceforge.io/) installation.

| Platform | Install |
|----------|---------|
| macOS (Homebrew) | `brew install maxima` |
| Ubuntu/Debian | `sudo apt install maxima` |
| Fedora | `sudo dnf install maxima` |
| Windows | See [Windows setup](#windows-setup) below |

### Windows Setup

On Windows you have three options for running Maxima:

**Option 1: Native Windows install (Local backend)**

Download and install Maxima from [sourceforge.net/projects/maxima](https://sourceforge.net/projects/maxima/). Aximar will automatically detect installations at `C:\maxima-*\bin\maxima.bat`. You can also set a custom path in Settings.

**Option 2: WSL backend**

If you use [Windows Subsystem for Linux](https://learn.microsoft.com/en-us/windows/wsl/install), you can run Maxima inside a WSL distribution:

1. Install Maxima in your WSL distro (e.g. `sudo apt install maxima`)
2. In Aximar, open Settings and set the backend to **WSL**
3. Select your distro from the dropdown — Aximar will show whether Maxima was found

Plotting works automatically — Aximar copies rendered SVGs from the WSL filesystem to a local temp directory.

**Option 3: Docker/Podman backend**

Run Maxima in a container for full isolation:

1. Install [Docker Desktop](https://www.docker.com/products/docker-desktop/) or [Podman](https://podman.io/)
2. Pull a Maxima image (e.g. `docker pull maxima/maxima`)
3. In Aximar, set the backend to **Docker**, choose your engine, and enter the image name

### Maxima detection order

Aximar looks for the `maxima` binary in these locations (local backend):

1. `AXIMAR_MAXIMA_PATH` environment variable
2. `/opt/homebrew/bin/maxima`, `/usr/local/bin/maxima`, `/usr/bin/maxima`
3. Windows: `C:\maxima-*\bin\maxima.bat`
4. Falls back to `maxima` on `PATH`

## Using the App

### Cells

- **Type a Maxima expression** in a cell (e.g. `integrate(x^2, x);`)
- **Shift+Enter** — evaluate the cell and create a new cell below
- **Ctrl/Cmd+Enter** — evaluate in place
- **+ Cell / + Markdown** — add code or markdown cells
- **Run All** — evaluate all cells in order
- **Restart** — restart the Maxima session (clears Maxima state, not your cells)

### Keyboard shortcuts

Hold Ctrl (or Cmd on Mac) to see all shortcuts. Key bindings include:

| Shortcut | Action |
|----------|--------|
| Shift+Enter | Evaluate and advance |
| Ctrl/Cmd+Enter | Evaluate in place |
| Ctrl/Cmd+K | Command palette |
| Ctrl/Cmd+Z | Undo |
| Ctrl/Cmd+Shift+Z | Redo |
| Ctrl/Cmd+F | Find |
| Ctrl/Cmd+Shift+F | Find & replace |
| Ctrl/Cmd+D | Delete cell |
| Ctrl/Cmd+Shift+Up/Down | Move cell |

### Settings

Open Settings from the toolbar to configure:

- **Theme** — auto, light, or dark
- **Cell style** — card or bracket
- **Backend** — local, Docker, or WSL
- **Font sizes** — editor and print
- **Markdown font** — sans-serif, serif, Computer Modern, or mono
- **Evaluation timeout** — 10s to 120s
- **Print margins** — top, bottom, left, right (mm)

### Logging

The status bar at the bottom of the app shows the most recent event. Click it to open the log window with two tabs:

- **App Log** — session events, evaluation results, warnings, and errors
- **Maxima Output** — raw stdin/stdout/stderr from the Maxima process (useful for debugging)

### Example expressions

```
integrate(x^2, x);
diff(sin(x)*cos(x), x);
solve(x^2 - 5*x + 6 = 0, x);
expand((a + b)^4);
factor(x^4 - 1);
taylor(exp(x), x, 0, 5);
plot2d(sin(x), [x, -%pi, %pi]);
```

## Building from Source

### Prerequisites

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) (stable toolchain)
- Tauri v2 system dependencies — see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)

### Development

```bash
npm install
npm run tauri dev
```

This starts both the Vite dev server (frontend hot-reload) and the Tauri Rust backend.

### Release build

```bash
npm run tauri build
```

Build artifacts are placed in `src-tauri/target/release/bundle/`:
- macOS: `.app` bundle and `.dmg`
- Linux: `.AppImage` and `.deb`
- Windows: `.msi` installer

### Running tests

```bash
# Rust unit tests
cd src-tauri && cargo test

# TypeScript type checking
npx tsc --noEmit
```

## Architecture

The app communicates with Maxima through a long-lived subprocess. The Rust backend manages this process, sending expressions via stdin and reading results from stdout using a sentinel-based protocol. See `docs/maxima-protocol.md` for details.

```
React Frontend  <->  Tauri IPC  <->  Rust Backend  <->  Maxima subprocess
                                                         (stdin/stdout)
```

| Layer | Technology |
|-------|-----------|
| Desktop shell | Tauri v2 |
| Frontend | React 19, TypeScript, Vite |
| Code editor | CodeMirror 6 |
| Math rendering | KaTeX |
| State management | Zustand |
| Subprocess I/O | tokio::process |

## Alternatives

- **[wxMaxima](https://wxmaxima-developers.github.io/wxmaxima/)** — the long-established GUI for Maxima, built with wxWidgets. wxMaxima offers a mature feature set including interactive animations with slider controls, `table_form()` for tabular data display, and notebook export to HTML/LaTeX. Aximar aims to provide a more modern interface and cross-platform experience via Tauri, but wxMaxima remains an excellent choice — especially if you need animation support or `.wxm`/`.wxmx` file compatibility.

## License

GPL-3.0-or-later
