# Multi-Notebook Support

## Overview

Aximar currently supports a single notebook per application instance. This document describes the design for supporting multiple simultaneous notebooks, each with its own isolated Maxima session.

## Current Architecture

| Component | Current State |
|---|---|
| **AppState** | Single `Arc<Mutex<Notebook>>` and single `Arc<SessionManager>` |
| **Tauri commands** | All operate on the single global notebook; no notebook ID parameter |
| **Frontend** | Single Zustand store (`notebookStore`) holding one set of cells |
| **Events** | Single `notebook-state-changed` event with no notebook context |
| **MCP server** | Shares AppState's single notebook and session in connected mode |
| **Output routing** | Single `CaptureOutputSink`; `MultiOutputSink` broadcasts to GUI + MCP |
| **Undo/redo** | Stacks live inside the single `Notebook` instance |

## Chosen Approach: Per-Notebook Sessions (Option A) with MCP Notebook ID (MCP-2)

### Notebook Model

Replace the single notebook and session with a `NotebookRegistry` — a map from `NotebookId` to `NotebookContext`:

```rust
struct NotebookContext {
    notebook: Arc<Mutex<Notebook>>,
    session: Arc<SessionManager>,
    capture_sink: Arc<CaptureOutputSink>,
    server_log: Arc<ServerLog>,
    /// File path if saved, None if untitled
    path: Option<PathBuf>,
}

struct NotebookRegistry {
    notebooks: HashMap<NotebookId, NotebookContext>,
    active: NotebookId,
}
```

Each notebook has its own Maxima process, undo/redo stacks, output capture, and variable state. Full isolation — no cross-notebook state leakage.

### Tauri Commands

All notebook and session commands gain a `notebook_id` parameter. The backend resolves the target `NotebookContext` from the registry before operating.

```rust
#[tauri::command]
pub async fn nb_add_cell(
    app: AppHandle,
    state: State<'_, AppState>,
    notebook_id: String,       // ← new
    cell_type: Option<String>,
    input: Option<String>,
    after_cell_id: Option<String>,
    before_cell_id: Option<String>,
) -> Result<NbAddCellResult, AppError> {
    let ctx = state.registry.get(&notebook_id)?;
    let mut nb = ctx.notebook.lock().await;
    let effect = nb.apply(NotebookCommand::AddCell { ... })?;
    emit_notebook_state(&app, &notebook_id, &nb, &effect);
    Ok(NbAddCellResult { cell_id })
}
```

New commands for notebook lifecycle:

- `nb_create()` → creates a new `NotebookContext`, starts a Maxima session, returns the ID
- `nb_close(notebook_id)` → stops the session, removes from registry
- `nb_list()` → returns IDs, titles, and paths of open notebooks
- `nb_set_active(notebook_id)` → sets the active notebook (for MCP default and frontend focus)

### Event Routing

The `notebook-state-changed` event payload gains a `notebook_id` field so the frontend knows which tab to update:

```rust
fn emit_notebook_state(
    app: &AppHandle,
    notebook_id: &str,
    nb: &Notebook,
    effect: &CommandEffect,
) {
    app.emit("notebook-state-changed", NotebookStatePayload {
        notebook_id: notebook_id.to_string(),
        cells: nb.cells_snapshot(),
        effect: effect.name(),
        cell_id: effect.cell_id(),
        can_undo: nb.can_undo(),
        can_redo: nb.can_redo(),
    });
}
```

Session status events similarly gain a notebook ID so the frontend shows per-tab status indicators.

### Output Sink Routing

Each `NotebookContext` has its own `CaptureOutputSink` and `TauriOutputSink`. In connected mode, each context builds its own `MultiOutputSink` that broadcasts to its GUI sink and MCP capture sink.

The `TauriOutputSink` needs to tag output events with the notebook ID so the frontend routes them to the correct tab:

```rust
app.emit("maxima-output", MaximaOutputPayload {
    notebook_id: notebook_id.to_string(),
    event: output_event,
});
```

### Frontend

#### Store Structure

Replace the single flat store with a per-notebook map. Options:

**A) Single store with notebook map** (recommended):

```typescript
interface NotebookState {
  notebooks: Map<string, NotebookTab>;
  activeNotebookId: string | null;
}

interface NotebookTab {
  id: string;
  title: string;
  cells: Cell[];
  sessionStatus: SessionStatus;
  activeCellId: string | null;
  canUndo: boolean;
  canRedo: boolean;
  isDirty: boolean;
}
```

Selectors derive the active notebook's state:

```typescript
const cells = useNotebookStore(s => s.notebooks.get(s.activeNotebookId)?.cells ?? []);
```

**B) Store per notebook** — each tab creates its own Zustand store. Simpler per-tab logic but harder to coordinate (tab bar needs to read all stores). Not recommended.

#### Tab Bar UI

A tab bar above the notebook area. Each tab shows:
- Notebook title (filename or "Untitled")
- Dirty indicator (unsaved changes)
- Session status indicator (dot: green=ready, yellow=busy, grey=stopped, red=error)
- Close button

Keyboard shortcuts: Cmd+T (new tab), Cmd+W (close tab), Cmd+Shift+[ / ] (switch tabs).

#### Component Changes

Most cell components receive their data from the active notebook slice and don't need structural changes. The main changes are:

- `App.tsx` — add tab bar, manage active notebook switching
- `notebookStore.ts` — restructure as above
- All `invoke()` calls — pass `notebookId` parameter
- Event listeners — filter by `notebook_id` in payload

### MCP Integration (MCP-2: Notebook ID in Tools)

Add an optional `notebook_id` parameter to all cell-manipulation and session tools. When omitted, defaults to the active notebook.

#### Tool Changes

```rust
#[tool(description = "Add a new cell to the notebook. Returns the new cell's ID.")]
async fn add_cell(&self, Parameters(params): Parameters<AddCellParams>) -> Result<String, String> {
    let notebook_id = params.notebook_id.as_deref()
        .unwrap_or(&self.active_notebook_id());
    let ctx = self.registry.get(notebook_id)?;
    // ... operate on ctx.notebook
}
```

Tools affected:
- `add_cell`, `delete_cell`, `update_cell`, `move_cell`, `run_cell`, `run_all_cells`
- `list_cells`, `get_cell`, `get_cell_output_log`
- `list_variables`, `kill_variable`
- `restart_session`, `get_session_status`
- `save_notebook`, `open_notebook`

Tools that remain global (no notebook ID needed):
- `evaluate_expression` — could target active notebook's session, or have its own scratch session
- `search_functions`, `get_function_docs`, `complete_function`, etc. (catalog tools)
- `list_templates`, `list_packages`, etc. (static data)

#### New MCP Tools

- `list_notebooks` — returns IDs, titles, and active status of all open notebooks
- `switch_notebook(notebook_id)` — changes the active notebook (affects default for tools that omit the ID)
- `create_notebook` — opens a new notebook tab
- `close_notebook(notebook_id)` — closes a notebook tab

#### MCP Server Instructions

Update the MCP instruction block to include the active notebook ID and list of open notebooks, so Claude can discover and target the right notebook:

```
activeNotebook: "abc123"
openNotebooks:
  - id: "abc123", title: "Untitled", path: null
  - id: "def456", title: "homework.ipynb", path: "/Users/..."
```

### Resource Considerations

Each notebook spawns a Maxima process (~30-50MB RSS). With 3-5 notebooks open this is manageable. Possible mitigations for resource pressure:

- **Idle session shutdown**: Stop Maxima processes for background notebooks after N minutes of inactivity. Restart on demand (cell execution or tab switch). Session state is lost but the notebook cells and outputs are preserved.
- **Session limit**: Cap the number of concurrent Maxima processes. Queue or refuse new notebooks beyond the limit.
- **Lazy start**: Don't start a Maxima process until the first cell is executed in a notebook.

### Migration Path

The refactor was staged as follows (all stages are now implemented):

1. **Backend registry** (`crates/aximar-core/src/registry.rs`): `NotebookRegistry` wrapping a map from `NotebookId` to `NotebookContext`. `NotebookContextRef` provides cheaply cloneable Arc-ref snapshots to avoid holding the registry lock during long operations.
2. **Tauri command plumbing**: All notebook/session/variable/evaluate commands accept `notebook_id: Option<String>`, defaulting to the active notebook via `registry.resolve()`. New lifecycle commands: `nb_create`, `nb_close`, `nb_list`, `nb_set_active`.
3. **Frontend tabs**: Zustand store restructured to `notebooks: Record<string, NotebookTab>` with `activeNotebookId`. Tab bar component (`TabBar.tsx`) with keyboard shortcuts. All invoke calls pass `notebookId`. Event listeners route by `notebook_id`.
4. **Per-notebook sessions**: `TauriOutputSink` tags output events with `notebook_id`. Each notebook context has its own `SessionManager`, `CaptureOutputSink`, and `ServerLog`.
5. **MCP tools**: `AximarMcpServer` stores `Arc<Mutex<NotebookRegistry>>` instead of individual fields. `ProcessSinkFactory` callback builds per-notebook output sinks. All notebook-bound tools accept optional `notebook_id`. New lifecycle tools: `list_notebooks`, `create_notebook`, `close_notebook`, `switch_notebook`. Server instructions updated with multi-notebook guidance.

Each stage was independently buildable and testable.

### Close Confirmation

When a notebook has unsaved changes, closing it requires user confirmation. This applies to all close paths:

- **GUI tab close button (x)** and **Cmd+W**: `handleCloseTab` in `TabBar.tsx` checks `isDirty` and shows a native dialog before proceeding.
- **MCP `close_notebook`**: In connected mode (GUI running), the backend emits a `close_requested` lifecycle event instead of closing directly. The frontend's `useNotebookEvents` hook receives this event and:
  - If the tab is clean: closes immediately via `nbClose()` + `removeTab()`.
  - If the tab is dirty: sets `closePending` on the tab (triggering a pulse animation), shows a confirmation dialog, and either completes or cancels the close.
- **Standalone mode** (headless MCP): `close_notebook` closes directly without confirmation since there is no GUI to prompt.

The `closePending` flag on `NotebookTab` prevents duplicate confirmation dialogs from concurrent close requests. After the dialog resolves, the handler re-reads store state to account for races (e.g. the user saved while the dialog was showing).

## Alternatives Considered

### Shared Maxima Session (Option B)

Multiple notebooks sharing one Maxima process. Rejected because Maxima has deep global state (facts database, loaded packages, rules, `%` and label counters, variable bindings). Clean context-switching between notebooks is essentially impossible without restarting the process, which defeats the purpose.

### Tauri Multi-Window (Option C)

Each notebook in a separate OS window with its own AppState. Provides natural isolation but adds complexity for shared resources (catalog, package data), window lifecycle management, and cross-window coordination. Also harder for users to manage than tabs.

### Active-Notebook Default with MCP Connection Pinning (MCP-3)

Pin the MCP connection to whichever notebook is active at connection time. Avoids changing tool signatures but is fragile across reconnections and doesn't let Claude explicitly target a specific notebook. MCP-2 (explicit notebook ID) is more robust and future-proof.

### One MCP Port Per Notebook (MCP-4)

Each notebook spawns its own MCP server on a dynamic port. Perfect isolation but breaks Claude Code's single well-known port assumption and adds port management complexity.
