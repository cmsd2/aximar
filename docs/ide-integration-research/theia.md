# Eclipse Theia

Analysis of using Eclipse Theia as a platform for building a custom Maxima
IDE with integrated notebook.

## What Theia Is

Theia is a **framework for building custom IDEs**, not a ready-made IDE.
(The Eclipse Foundation also ships a "Theia IDE" product built on the
framework, but that's a separate thing.)

You compose an application by selecting Theia packages (file explorer,
terminal, editor, debugger, git panel, etc.), add your own extensions, and
compile the whole thing. Written in TypeScript, it runs as a Node.js
backend + browser frontend.

**Key difference from VS Code:** Every UI piece is a replaceable module via
InversifyJS dependency injection. You can unbind, replace, or extend any
built-in component. VS Code's extension API is a controlled surface; Theia
gives you full internal access.

## Architecture

- **Backend:** Node.js (Express server, handles filesystem, processes,
  language servers, git)
- **Frontend:** Browser application communicating with backend over
  WebSockets (JSON-RPC) and HTTP
- **Desktop packaging:** Electron (not Tauri — this is a hard constraint)
- **Browser-only mode:** Available since Theia 1.46, but limited (no real
  terminal, limited git, no native process access)

## What You Get for Free

| Feature | Package | Quality |
|---|---|---|
| File explorer | `@theia/filesystem`, `@theia/navigator` | Production-quality, comparable to VS Code |
| Git integration | `@theia/git` | Full: status, diff, stage/unstage, commit, branches. Uses bundled `dugite` |
| Terminal | `@theia/terminal` | xterm.js-based, works in Electron |
| Debug panel | `@theia/debug` | Full DAP-based debugger UI |
| Search | `@theia/search-in-workspace` | ripgrep-based, with replace |
| Monaco editor | Built-in | Same editor component as VS Code |
| Keybindings/settings | Built-in | VS Code-compatible |
| Multi-tab editor | Built-in | Split views, dirty indicators, etc. |

## VS Code Extension Compatibility

Theia declared "full compatibility with the VS Code Extension API" in
December 2023. In practice:

- **All API namespaces are covered** — no missing stubs that crash on import
- **Most popular extensions work:** language extensions, linters, formatters,
  theme extensions
- **Notebook API:** Implemented since Theia 1.48 (early 2024), supports
  `.ipynb`, cell execution, output rendering, drag-and-drop
- **Debug Adapter Protocol:** Full support — your `maxima-dap` would work
- **WebView API:** Supported
- **Gaps:** AI/chat APIs (VS Code 1.80+) may have issues; Theia typically
  lags VS Code by ~1 month

**Your existing VS Code extension (LSP + DAP + MCP) would work as-is.**

## Customization

This is Theia's strongest selling point:

- **Replace any view** — unbind the default file tree, bind your own
- **Custom widgets** — implement `Widget` subclass with React/Preact/Lit
- **Custom editors** — `WidgetOpenHandler` for domain-specific file types
- **Theming** — VS Code theme format (JSON color tokens), full CSS override
  access
- **Branding** — logo, window title, about dialog — all configurable

## The Notebook Question

Two approaches for the Aximar notebook in Theia:

### A. Use Theia's built-in notebook (VS Code Notebook API)

Same capabilities and limitations as [vscode-notebook-api.md](vscode-notebook-api.md).
Your notebook controller talks to Maxima, renderers handle KaTeX/Plotly.

**Pros:** Least custom code, VS Code notebook extensions work.
**Cons:** Single-column layout, no custom cell types, limited undo.

### B. Build a custom Theia widget

Port your existing React notebook UI into a Theia `Widget`. The widget
registers as a custom editor for `.ipynb` files (or a custom format).

**Pros:** Full control over the notebook UX — preserves everything that
makes Aximar's notebook special.
**Cons:** More work (6-10 weeks), no VS Code notebook extension
compatibility.

## Tauri Compatibility

**Theia's desktop story is Electron-only.** There is no supported way to run
Theia inside Tauri.

The theoretical workaround — run Theia's Node.js backend as a sidecar,
point Tauri's webview at `http://localhost:3000` — is architecturally messy:

- Two app frameworks fighting over keyboard shortcuts, focus, theming
- Two process trees to manage
- No shared state between Tauri's Rust backend and Theia's Node.js backend
- Entirely unsupported by Theia

**Adopting Theia means abandoning Tauri.** Your Rust crates (`aximar-core`,
`aximar-mcp`) would become sidecar processes called from Theia's Node.js
backend via stdio/IPC.

## Effort Estimate (Theia-based Aximar)

| Work Item | Effort |
|---|---|
| Theia app scaffold + package selection | 1-2 weeks |
| Custom notebook widget (porting React notebook UI) | 6-10 weeks |
| Maxima kernel integration (session management as sidecar) | 2-4 weeks |
| MCP server integration | 1-2 weeks |
| Branding/theming | 1 week |
| Electron packaging/distribution | 1 week |
| **Total** | **~12-20 weeks** |

## Hidden Costs

- **InversifyJS learning curve:** Budget 1 week to understand the DI system
  well enough to be productive
- **Build times:** Theia is a large monorepo; clean builds take several
  minutes. Requires `watch` mode for iterative development
- **Documentation:** Covers concepts but skips practical details. You will
  read Theia source code and GitHub discussions extensively
- **Monthly releases:** Theia releases monthly with possible breaking changes
  in experimental APIs. Stable APIs follow semver

## Bundle Size and Performance

- **Install size:** ~400-600 MB (vs Tauri's ~30-50 MB)
- **Startup time:** Comparable to VS Code after performance improvements in
  Theia 1.38-1.40. Slower than Tauri
- **Memory:** ~500-650 MB at runtime (comparable to VS Code)

## Production Examples

| Product | Company | Domain |
|---|---|---|
| Arduino IDE 2.x | Arduino | Embedded/maker (millions of users) |
| Code Composer Studio | Texas Instruments | Professional embedded |
| Vitis IDE | AMD/Xilinx | FPGA/SoC development |
| STM32CubeMX2 | STMicroelectronics | MCU configuration |
| Sokatoa | Samsung | GPU profiling |
| Martini Designer | Lonti | Enterprise integration |
| Business Application Studio | SAP | Cloud development |
| Artemis | TU Munich | CS education |
| Google Cloud Shell Editor | Google | Cloud shell IDE |

These demonstrate that production-quality, domain-specific IDEs built on
Theia are achievable and ship to real users.

## Assessment for Aximar

Theia is compelling if you want **all IDE features in one window** without
building them yourself. The non-notebook parts (file explorer, git, terminal,
debugger) are free and production-quality.

The cost is significant:

- Abandoning Tauri and its small-binary/native-performance story
- Porting the notebook UI to a Theia widget
- Restructuring the Rust backend as a sidecar
- Taking on Electron's resource footprint
- InversifyJS + Theia learning curve

Theia makes most sense if you're starting from zero and want an IDE with a
custom notebook added. For Aximar — where the notebook is already built and
polished in Tauri — the migration cost is harder to justify unless the IDE
features are truly essential.
