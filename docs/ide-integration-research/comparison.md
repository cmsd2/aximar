# Comparison: Monaco DIY vs Theia vs VS Code Notebook

Side-by-side comparison of the three main approaches for integrating IDE
features with Aximar's notebook.

## Feature Matrix

| Feature | Monaco in Tauri | Theia | VS Code Notebook |
|---|---|---|---|
| Code editing with LSP | Yes (monaco-languageclient) | Yes (built-in Monaco) | Yes (full Monaco in cells) |
| File explorer | Build it (~2-3 weeks) | Free | Free (VS Code's) |
| Git integration | Build it (~4-8 weeks) | Free | Free (VS Code's) |
| Debug panel | Build it (~4-6 weeks) | Free | Free (VS Code's) |
| Terminal | Build it (~1-2 weeks) | Free | Free (VS Code's) |
| Search across files | Build it (~1-2 weeks) | Free | Free (VS Code's) |
| Notebook UX | Keep existing (best) | Port to widget (~6-10 weeks) | VS Code notebook (constrained) |
| KaTeX rendering | Keep existing | Port to widget | Custom renderer (good) |
| Plotly charts | Keep existing | Port to widget | Custom renderer (good) |
| Variable inspector | Keep existing | Port to webview | Webview panel (good) |
| Custom cell types | Yes | Yes (custom widget) | No (Code + Markup only) |
| Custom layout | Full control | Full control (custom widget) | Single-column only |
| Notebook undo/redo | Keep existing | Port to widget | Limited (per-cell only) |
| .ipynb compatibility | Keep existing | Implement in widget | NotebookSerializer |
| VS Code extensions | No | Yes (most work) | Yes (all) |
| Existing extension reuse | Partial (LSP binary) | Full (extension runs as-is) | Full |

## Effort Comparison

| Approach | Effort to Usable | Effort to Polished | Ongoing Maintenance |
|---|---|---|---|
| Monaco in Tauri | 14-24 weeks | 6+ months | High (every IDE feature is yours) |
| Theia | 12-20 weeks | 4-6 months | Medium (Theia provides IDE; notebook is yours) |
| VS Code Notebook | 6-10 weeks | 3-4 months | Low (VS Code provides everything; you maintain kernel + renderers) |
| MCP Bridge (option 1) | 1-2 weeks | 2-3 weeks | Very low |

## Trade-off Analysis

### Monaco in Tauri

**Choose if:** You want a single cohesive app and are willing to invest
heavily in building and maintaining IDE features. You value Tauri's small
binary size and native performance.

**Avoid if:** You need git, debugging, and terminal to be as good as VS Code.
Each feature you build is a maintenance commitment forever.

**Risk:** Feature creep. Once you add a file tree, users want fuzzy file
opening. Once you add git, users want inline blame. You're pulled into
building a general-purpose IDE, which is not the product.

### Theia

**Choose if:** You've decided the notebook MUST live inside a full IDE, and
you're willing to abandon Tauri/Electron trade-off. You want professional
IDE features without building them.

**Avoid if:** You value Tauri's small footprint, or the Electron migration
cost is prohibitive for your project.

**Risk:** Architectural migration. Tauri → Electron, Rust backend → Node.js
sidecar, React notebook → Theia widget. This is a significant rewrite of the
application shell, even if the core logic in `aximar-core` is preserved.

### VS Code Notebook

**Choose if:** You're comfortable with the VS Code notebook UX (single-column,
fixed chrome) and primarily want the integration value of notebook + IDE in
one window.

**Avoid if:** You need the notebook to look and feel like Aximar's custom UI.
The layout and chrome constraints are hard limits.

**Risk:** Disappointment. After building it, the notebook may feel like a
downgrade from Aximar's purpose-built UX, even though the IDE integration
is genuinely better.

### MCP Bridge (Baseline)

**Choose if:** You want maximum value for minimum effort and are comfortable
with two windows.

**Avoid if:** Context switching between apps is genuinely unacceptable for
your workflow.

**Risk:** Almost none. This is the safe baseline that delivers value
regardless of which other option you pursue later.

## Decision Framework

```
Do you need notebook + IDE in ONE window?
├── No → Option 1 (MCP Bridge). Done.
└── Yes
    ├── Is the VS Code notebook UX acceptable?
    │   ├── Yes → Option 2 (VS Code Notebook)
    │   └── No (need custom notebook UX)
    │       ├── Is abandoning Tauri acceptable?
    │       │   ├── Yes → Option 3b (Theia)
    │       │   └── No → Option 3a (Monaco in Tauri)
    │       │       └── But limit scope! Minimal file editor,
    │       │           not a full IDE. Use MCP bridge for
    │       │           git/debug/terminal via VS Code.
    │       └── (Consider: do you really need it in one window?)
    └── Start with MCP Bridge regardless — it's useful no matter what.
```

## Summary

The approaches form a spectrum of effort vs integration:

```
Less effort                                              More effort
More separation                                     More integration

  MCP Bridge → VS Code Notebook → Theia → Monaco DIY
   1-2 wks      6-10 wks        12-20 wks   14-24 wks
```

Every option to the right of MCP Bridge trades significant engineering
effort for tighter integration. The question is whether that integration
justifies the cost, given that two purpose-built tools cooperating via MCP
can deliver most of the same value.
