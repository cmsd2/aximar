# IDE Integration Research

Research into options for combining Aximar's notebook experience with full IDE
capabilities for editing `.mac` files.

## Context

Aximar is a standalone Tauri desktop notebook app, excellent for interactive
math/science work (KaTeX rendering, Plotly plots, variable inspector, command
palette). A sibling VS Code extension (`../maxima-extension`) provides IDE
features for `.mac` files (LSP, DAP, syntax highlighting, debugging). The
question: how to get the best of both worlds.

## Documents

- [**options-overview.md**](options-overview.md) — Summary of all approaches
  considered, with pros/cons and recommendations
- [**vscode-notebook-api.md**](vscode-notebook-api.md) — Deep dive into VS
  Code's Notebook API: capabilities, hard limitations, and what a Maxima
  notebook extension could achieve
- [**monaco-in-tauri.md**](monaco-in-tauri.md) — Embedding Monaco Editor in
  the Tauri app: what you get, what you'd need to build, effort estimates
- [**theia.md**](theia.md) — Eclipse Theia as a platform for a custom
  Maxima IDE: architecture, VS Code compatibility, real-world examples, effort
  estimates
- [**comparison.md**](comparison.md) — Side-by-side comparison of Monaco DIY
  vs Theia vs VS Code Notebook API, with effort estimates and trade-offs
