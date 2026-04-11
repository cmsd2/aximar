# VS Code Notebook API

Deep dive into VS Code's Notebook API capabilities and limitations for
building a Maxima math/science notebook experience.

## Architecture Overview

The VS Code Notebook API has three main components:

- **NotebookSerializer** — reads/writes notebook files (e.g., `.ipynb`)
- **NotebookController** — handles cell execution (your kernel)
- **NotebookRenderer** — renders cell outputs (KaTeX, Plotly, etc.)

## NotebookController

```typescript
const controller = vscode.notebooks.createNotebookController(
  'maxima-controller',
  'maxima-notebook',
  'Maxima',
  executeHandler
);
```

The controller receives cells to execute, creates `NotebookCellExecution`
objects, and reports results via `replaceOutput()` / `appendOutput()`.

**What you control:**

- Execution order counter
- Start/end timestamps
- Output replacement/appending
- Success/failure signaling
- Interrupt handling (for Maxima's `$abort`)
- Continuous execution mode (for streaming results)

**What you cannot control:**

- Cannot queue or reorder execution requests
- No explicit cell lifecycle hooks (use
  `onDidChangeNotebookDocument` instead)
- No built-in kernel state inspection — you manage the Maxima process

## NotebookSerializer

```typescript
vscode.notebooks.registerNotebookSerializer(
  'maxima-notebook',
  new MaximaNotebookSerializer()
);
```

Implements `deserializeNotebook` and `serializeNotebook` to read/write
`.ipynb` format. The `NotebookData` structure maps well to Jupyter's format:
cells with kind, source, language, outputs, and metadata.

**Key points:**

- Outputs are NOT persisted automatically — you must serialize them
  explicitly
- Metadata is a plain object, store anything you need
- Can be registered for `.ipynb` files directly

## NotebookRenderer — The Key to Rich Output

All notebook outputs render inside a **single shared iframe** (not one per
cell). Renderers have full DOM and JS access within their output element.

### Registration

```json
{
  "contributes": {
    "notebookRenderer": [{
      "id": "maxima-latex",
      "entrypoint": "./out/renderer.js",
      "mimeTypes": ["text/latex", "application/x-maxima-output"]
    }]
  }
}
```

### Renderer Script

```typescript
export const activate: ActivationFunction = (context) => ({
  renderOutputItem(outputItem, element) {
    // Full DOM access within `element`
    // Bundle KaTeX, Plotly, or any JS library
    const latex = outputItem.text();
    katex.render(latex, element);
  },
  disposeOutputItem(id) { /* cleanup */ }
});
```

### What renderers CAN do

- Render arbitrary HTML and execute arbitrary JavaScript
- Use Plotly, D3, KaTeX, MathJax, Three.js — any JS library
- Use `fetch()`, WebSockets, async operations
- Access VS Code theme CSS variables (`var(--vscode-editor-background)`)
- Communicate back to the extension host via `postMessage`

### What renderers CANNOT do

- Access VS Code APIs directly (must use message passing)
- Isolate from other cell outputs (shared iframe, shared `window`)
- Inject CSS into the notebook chrome

### Renderer Messaging

**Renderer to extension host:**

```typescript
// In renderer:
context.postMessage({ type: 'plot-clicked', data: '...' });

// In extension:
const messaging = vscode.notebooks.createRendererMessaging('maxima-latex');
messaging.onDidReceiveMessage(({ editor, message }) => { /* handle */ });
```

**Controller preloads (kernel-renderer communication):**

```typescript
controller.rendererScripts = [
  new vscode.NotebookRendererScript(uri)
];
// In preload: postKernelMessage() / onDidReceiveKernelMessage()
```

## Cell Editor

Each code cell uses a **full Monaco editor** — the same editor used for
normal files. This means:

- Full LSP support (completions, hover, diagnostics, go-to-definition)
- The Jupyter extension implements a "concat-document middleware" that
  virtually concatenates cells into one document for the language server
- Language per cell via `languageId`
- Editor pooling for performance (transparent to extensions)

For Maxima, you would either:

1. Register the LSP to handle the `maxima` language ID directly, or
2. Implement concat-document middleware to give the LSP a coherent view of
   all cells

## Layout and Customization

### What you CAN customize

| Feature | How |
|---|---|
| Notebook toolbar buttons | `contributes.menus` → `notebook/toolbar` |
| Per-cell toolbar buttons | `contributes.menus` → `notebook/cell/title` |
| Per-cell status bar items | `registerNotebookCellStatusBarItemProvider` |
| Side panels (variable inspector, docs) | `registerWebviewViewProvider` |
| Full webview panels (data viewer, plot viewer) | `createWebviewPanel` |

### What you CANNOT customize

- **Layout is fixed:** single-column scrollable list, no horizontal splits
- **Cell types:** only Code and Markup (no custom cell kinds)
- **Notebook chrome:** cannot restyle cell borders, gutters, or notebook frame
- **Markdown renderer:** built-in for markup cells, cannot be replaced
- **Drag-and-drop:** built-in, cannot hook into or prevent
- **Keyboard shortcuts:** cannot override built-in notebook keybindings
  (Escape/Enter for command/edit mode)

## Undo/Redo

- Cell text edits: undoable within that cell's Monaco editor
- Structural edits (add/delete/move cell): undoable via `NotebookEdit`
- Cell execution: NOT undoable
- No notebook-global undo history crossing cell boundaries
- This is a significant gap vs Aximar's unified
  `Notebook::apply(NotebookCommand)` undo stack

## Supplementary UI via Webview Panels

The Jupyter extension demonstrates the pattern:

- **Variable Inspector**: `WebviewView` in the sidebar, built with React,
  updates on cell execution
- **Data Viewer**: full `WebviewPanel` (tab) for DataFrames
- **Plot Viewer**: `WebviewPanel` for enlarged plot viewing

Communication flows through the extension host — webview panels cannot talk
directly to notebook renderers.

## Theming

- Cell editors inherit VS Code's active color theme automatically
- Renderers receive `--vscode-*` CSS variables matching the active theme
- No API to inject custom CSS into the notebook frame
- Product icon themes can be contributed

## Hard Limitations Summary

1. No custom cell types (only Code and Markup)
2. Single-column layout, cannot be changed
3. No access to notebook DOM from extensions
4. Renderer iframe isolated from extension host
5. No notebook-global undo across cells
6. Cannot override built-in notebook keyboard shortcuts
7. Cannot intercept drag-and-drop reordering
8. Output rendering is per-MIME-type, not per-cell
9. All outputs share one iframe (buggy renderer affects all)
10. No persistent "notebook-global" UI area (header/footer)

## Assessment for Maxima

You can build a **functionally equivalent** notebook: same math rendering,
same plot interactivity, better LSP features (Monaco > CodeMirror for LSP),
same `.ipynb` format. The Jupyter extension proves the ceiling is high.

It will **feel like a VS Code notebook**, not like Aximar. The single-column
layout, the VS Code chrome, the two-mode keyboard model — those are VS Code's
personality. You're decorating a room in someone else's house.

The real value isn't matching the Aximar UX — it's getting the notebook
experience inside the same window as `.mac` file editing, debugging, git,
and terminal. The integration value may outweigh the UX polish gap.

**Estimated effort:** 6-10 weeks for a polished implementation including
custom renderers (KaTeX + Plotly), kernel management, `.ipynb`
serialization, and supplementary panels (variable inspector, docs).
