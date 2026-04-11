# Monaco Editor in Tauri

Analysis of embedding Monaco Editor in the Aximar Tauri app to add IDE-style
code editing alongside the notebook.

## What Monaco Gives You

[Monaco Editor](https://microsoft.github.io/monaco-editor/) is the editor
component extracted from VS Code. It's a standalone npm package designed for
embedding.

**Included:**

- Full code editor (syntax highlighting, multi-cursor, minimap, folding,
  find & replace, command palette)
- LSP support via
  [`monaco-languageclient`](https://github.com/TypeFox/monaco-languageclient)
  — connects to `maxima-lsp` over stdio or WebSocket
- Completions, hover, diagnostics, go-to-definition, find references,
  signature help — all via LSP
- Theme support (VS Code themes can be adapted)

**NOT included (these are VS Code, not Monaco):**

- Extension ecosystem
- File explorer
- Git integration
- Debugger UI
- Terminal
- Search across files
- Keybindings framework
- Settings UI
- Multi-tab editor management

## What You'd Need to Build

| Feature | Effort | Notes |
|---|---|---|
| File tree panel | 2-3 weeks | Recursive directory listing via Tauri fs APIs, watch for changes, rename/delete/create, file icons, lazy loading, drag-drop |
| Git integration | 4-8 weeks | Shell out to `git` CLI. Basic "changed files + commit" panel: 2 weeks. Diff view, stage/unstage, branch switching, log: 4+ more weeks. Anything approaching GitLens quality: months |
| Debug panel | 4-6 weeks | DAP client in the frontend talking to `maxima-dap`. Breakpoint gutter markers, call stack tree, variables panel, debug toolbar (step/continue/stop), debug console. Protocol is well-defined but UI is substantial |
| Terminal | 1-2 weeks | Embed xterm.js + Tauri shell sidecar. Straightforward |
| Multi-tab editor | 2-3 weeks | Tab bar, dirty indicators, split views, save/close confirmation flows |
| Search across files | 1-2 weeks | ripgrep sidecar + results UI with file/line navigation |
| **Total** | **~14-24 weeks** | To reach "usable" quality |

## The 80/20 Problem

Each feature starts simple but users quickly miss the polish:

- File tree without fuzzy file opening (`Ctrl+P`)
- Git panel without inline blame or three-way merge
- Search without replace-in-files with preview
- Debug panel without conditional breakpoints or watch expressions
- Terminal without split panes or shell integration

VS Code has had hundreds of engineer-years invested in these features. Your
implementations will always be playing catch-up.

## Integration with Existing Aximar UI

The appealing part: Monaco integrates into your existing React UI naturally.

- Shared theme system
- Split views between notebook and code editor
- Drag-and-drop between notebook cells and file editor
- Unified command palette
- Consistent keyboard shortcuts

This cohesive integration is what you CAN'T get from VS Code's notebook API
or from Theia.

## Alternatives to Full IDE Features

Instead of building everything, consider a minimal "code editor" mode:

1. **Monaco editor tabs** for `.mac` files (with LSP via
   `monaco-languageclient`)
2. **Simple file list** (not a full tree — just recent files / project files)
3. **"Run File" button** that batch-loads the file in the Maxima session
4. **Skip git, terminal, search** — users already have VS Code for that

This gives you the core editing experience in ~4-6 weeks without the IDE
maintenance burden. Users who need git/terminal/search open VS Code alongside
Aximar (the MCP bridge makes this seamless).

## Who Uses This Approach

- **Gitpod** (historically, before switching to VS Code Server)
- **Jupyter Lab** (Monaco-based code cells, custom file browser)
- **Google Colab** (Monaco cells, minimal surrounding IDE)
- **Replit** (Monaco editor + custom file tree + terminal)

All of these invested heavily in the surrounding IDE features over years.

## Recommendation

If you embed Monaco, **resist the urge to build a full IDE**. Start with
just the code editor + LSP and a minimal file list. Accept that VS Code
handles everything else. The MCP bridge connects the two apps.

If you find yourself needing file explorer + git + debug + terminal in Aximar,
you've crossed the threshold where Theia (which provides all of these) becomes
the less-costly option — see [theia.md](theia.md).
