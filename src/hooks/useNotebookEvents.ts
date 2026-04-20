import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask } from "@tauri-apps/plugin-dialog";
import { useNotebookStore } from "../store/notebookStore";
import { nbCreate, nbGetState, nbList, nbClose, nbLoadCells, getInitialFileArgs } from "../lib/notebook-commands";
import { openFilePath, parseNbformatOutputs } from "../lib/notebooks-client";
import { markDirty, cleanup } from "../lib/dirty-inputs";
import type { CellOutput, CellStatus, CellType } from "../types/notebook";
import type { SessionStatus } from "../types/maxima";

/** Returns true when this window is the primary (first) window. */
function isMainWindow(): boolean {
  return getCurrentWindow().label === "main";
}

interface SyncCellOutput {
  text_output: string;
  latex: string | null;
  plot_svg: string | null;
  plot_data: string | null;
  error: string | null;
  is_error: boolean;
  duration_ms: number;
  output_label: string | null;
  execution_count: number | null;
}

interface SyncCell {
  id: string;
  cell_type: string;
  input: string;
  output?: SyncCellOutput | null;
  status?: string | null;
}

interface NotebookStateEvent {
  notebook_id: string;
  effect: string;
  cell_id: string | null;
  cells: SyncCell[];
  can_undo: boolean;
  can_redo: boolean;
  trusted: boolean;
}

/**
 * Focus the CodeMirror editor inside a cell container.
 * Retries briefly to handle the case where React hasn't rendered the cell yet.
 */
function focusCellEditor(cellId: string, retries = 3) {
  const container = document.querySelector(`[data-cell-id="${cellId}"]`);
  const cmContent = container?.querySelector<HTMLElement>(".cm-content");
  if (cmContent) {
    cmContent.focus();
  } else if (retries > 0) {
    requestAnimationFrame(() => focusCellEditor(cellId, retries - 1));
  }
}

function mapSyncCells(syncCells: SyncCell[]) {
  return syncCells.map((sc) => {
    let output: CellOutput | null = null;
    if (sc.output) {
      output = {
        textOutput: sc.output.text_output,
        latex: sc.output.latex,
        plotSvg: sc.output.plot_svg,
        plotData: sc.output.plot_data,
        error: sc.output.error,
        errorInfo: null,
        isError: sc.output.is_error,
        durationMs: sc.output.duration_ms,
        outputLabel: sc.output.output_label,
        executionCount: sc.output.execution_count,
      };
    }
    return {
      id: sc.id,
      cellType: sc.cell_type as CellType,
      input: sc.input,
      output,
      status: (sc.status as CellStatus) ?? "idle",
    };
  });
}

/**
 * Open a file from a path into the appropriate tab type.
 * For notebooks (.macnb/.ipynb), creates a backend notebook and loads cells.
 * For JSON files, creates a frontend-only plot tab.
 */
async function openFileAsTab(path: string): Promise<void> {
  const result = await openFilePath(path);
  const store = useNotebookStore.getState();

  if (result.type === "plot") {
    const title = path.split("/").pop()?.split("\\").pop() ?? "Plot";
    const id = `plot-${crypto.randomUUID()}`;
    store.addPlotTab(id, title, result.plotData, result.path);
  } else {
    // Create a backend notebook and load cells into it
    const { notebook_id } = await nbCreate();
    const title = path.split("/").pop()?.split("\\").pop() ?? "Untitled";
    store.addTab(notebook_id, title);
    store.setActiveTab(notebook_id);

    const cells = result.notebook.cells
      .filter((c) => c.cell_type !== "raw")
      .map((c) => {
        const parsed = c.cell_type === "code" ? parseNbformatOutputs(c) : null;
        return {
          id: crypto.randomUUID(),
          cell_type: c.cell_type === "markdown" ? "markdown" : "code",
          input: typeof c.source === "string" ? c.source : (c.source as string[]).join(""),
          output: parsed ?? undefined,
        };
      });
    await nbLoadCells(cells, notebook_id);
    // nbLoadCells triggers a backend event that updates the store via notebook-state-changed

    // Set file path so Save works
    const storeAfterLoad = useNotebookStore.getState();
    const tab = storeAfterLoad.notebooks[notebook_id];
    if (tab && tab.type === "notebook") {
      useNotebookStore.setState({
        notebooks: {
          ...storeAfterLoad.notebooks,
          [notebook_id]: { ...tab, filePath: path, title, isDirty: false },
        },
      });
    }
  }
}

/**
 * Hook that listens for `notebook-state-changed` events from the backend
 * and applies them to the frontend Zustand store. Also handles debounced
 * input sync from local edits to the backend.
 */
export function useNotebookEvents() {

  // --- Initial tab setup: discover notebooks from backend ---
  // Use ignore flag to handle React strict mode double-mounting — prevents
  // creating duplicate notebooks when the effect fires twice.
  useEffect(() => {
    let ignore = false;

    if (isMainWindow()) {
      // Main window: check for CLI file arguments first
      getInitialFileArgs().then(async (fileArgs) => {
        if (ignore) return;

        if (fileArgs && fileArgs.length > 0) {
          // Open files from CLI args — don't create a default notebook
          for (const path of fileArgs) {
            try {
              await openFileAsTab(path);
            } catch (e) {
              console.warn("Failed to open file from CLI:", path, e);
            }
          }
          return;
        }

        // No file args: discover all existing notebooks from the backend
        const notebooks = await nbList();
        if (ignore) return;
        const store = useNotebookStore.getState();
        for (const nb of notebooks) {
          if (!store.notebooks[nb.id]) {
            store.addTab(nb.id, nb.title);
          }
        }
        const active = notebooks.find((nb) => nb.is_active);
        if (active) {
          useNotebookStore.getState().setActiveTab(active.id);
        }
        for (const nb of notebooks) {
          nbGetState(nb.id).then((state) => {
            if (ignore) return;
            const { notebook_id, cells: syncCells, effect, cell_id, can_undo, can_redo, trusted } = state;
            useNotebookStore.getState().applyBackendState(
              notebook_id,
              mapSyncCells(syncCells),
              effect,
              cell_id ?? undefined,
              can_undo,
              can_redo,
              trusted
            );
          });
        }
      });
    } else {
      // Secondary window: check for a notebook ID in the URL, or create a fresh one
      const params = new URLSearchParams(window.location.search);
      const notebookParam = params.get("notebook");

      const adoptNotebook = (id: string) => {
        const store = useNotebookStore.getState();
        if (!store.notebooks[id]) {
          store.addTab(id);
        }
        store.setActiveTab(id);
        nbGetState(id).then((state) => {
          if (ignore) return;
          useNotebookStore.getState().applyBackendState(
            state.notebook_id,
            mapSyncCells(state.cells),
            state.effect,
            state.cell_id ?? undefined,
            state.can_undo,
            state.can_redo,
            state.trusted,
          );
        });
      };

      if (notebookParam) {
        adoptNotebook(notebookParam);
      } else {
        nbCreate().then((result) => {
          if (ignore) return;
          adoptNotebook(result.notebook_id);
        });
      }
    }

    return () => { ignore = true; };
  }, []);

  // --- Backend → Frontend: listen for state changes ---
  useEffect(() => {
    const unlisten = listen<NotebookStateEvent>(
      "notebook-state-changed",
      (event) => {
        const { notebook_id, cells: syncCells, effect, cell_id, can_undo, can_redo, trusted } =
          event.payload;

        // Ensure tab exists (e.g. created by MCP) — only auto-adopt in the main window
        const store = useNotebookStore.getState();
        if (!store.notebooks[notebook_id]) {
          if (isMainWindow()) {
            store.addTab(notebook_id);
          } else {
            return; // Ignore events for notebooks not in this window
          }
        }

        store.applyBackendState(
          notebook_id,
          mapSyncCells(syncCells),
          effect,
          cell_id ?? undefined,
          can_undo,
          can_redo,
          trusted
        );

        // Focus the newly added cell after React renders it,
        // but only if it's in the active notebook
        if (
          effect === "cell_added" &&
          cell_id &&
          notebook_id === useNotebookStore.getState().activeNotebookId
        ) {
          requestAnimationFrame(() => focusCellEditor(cell_id));
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // --- Backend → Frontend: listen for notebook lifecycle events (MCP) ---
  useEffect(() => {
    const unlisten = listen<{ notebook_id: string; event: string }>(
      "notebook-lifecycle",
      (event) => {
        const { notebook_id, event: eventType } = event.payload;
        const store = useNotebookStore.getState();

        if (eventType === "created") {
          // Only auto-adopt MCP-created notebooks in the main window
          if (!store.notebooks[notebook_id] && isMainWindow()) {
            store.addTab(notebook_id);
          }
        } else if (eventType === "closed") {
          if (store.notebooks[notebook_id]) {
            store.removeTab(notebook_id);
          }
        } else if (eventType === "close_requested") {
          const tab = store.notebooks[notebook_id];
          // Skip if tab doesn't exist, is already pending close, or is the last tab
          if (!tab || tab.closePending || Object.keys(store.notebooks).length <= 1) return;

          if (!tab.isDirty) {
            // Clean tab — close immediately
            nbClose(notebook_id).then(() => {
              useNotebookStore.getState().removeTab(notebook_id);
            }).catch((e) => console.warn("Failed to close notebook:", e));
          } else {
            // Dirty tab — prompt user
            store.setClosePending(notebook_id, true);
            ask("You have unsaved changes. Close without saving?", {
              title: "Unsaved Changes",
              kind: "warning",
            }).then((confirmed) => {
              // Re-read store state after await to handle races (e.g. user saved while dialog was showing)
              const current = useNotebookStore.getState();
              const currentTab = current.notebooks[notebook_id];
              if (!currentTab) return; // Tab was removed while dialog was open

              if (confirmed) {
                nbClose(notebook_id).then(() => {
                  useNotebookStore.getState().removeTab(notebook_id);
                }).catch((e) => console.warn("Failed to close notebook:", e));
              } else {
                current.setClosePending(notebook_id, false);
              }
            });
          }
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // --- Backend → Frontend: listen for session status changes (auto-start) ---
  useEffect(() => {
    const unlisten = listen<{ notebook_id: string; status: SessionStatus }>(
      "session-status-changed",
      (event) => {
        const { notebook_id, status } = event.payload;
        useNotebookStore.getState().setSessionStatusForNotebook(notebook_id, status);
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // --- Backend → Frontend: open files from second-instance CLI args ---
  useEffect(() => {
    const unlisten = listen<string[]>(
      "open-files",
      async (event) => {
        for (const path of event.payload) {
          try {
            await openFileAsTab(path);
          } catch (e) {
            console.warn("Failed to open file:", path, e);
          }
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // --- Frontend → Backend: debounced input sync ---
  useEffect(() => {
    const unsubscribe = useNotebookStore.subscribe((state, prevState) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return;
      const tab = state.notebooks[nbId];
      const prevTab = prevState.notebooks[nbId];
      if (!tab || !prevTab) return;
      if (tab.type !== "notebook" || prevTab.type !== "notebook") return;

      // Find cells with changed input
      for (const cell of tab.cells) {
        const prev = prevTab.cells.find((c: { id: string }) => c.id === cell.id);
        if (prev && prev.input !== cell.input) {
          markDirty(cell.id, cell.input);
        }
      }
    });

    return () => {
      unsubscribe();
      cleanup();
    };
  }, []);
}
