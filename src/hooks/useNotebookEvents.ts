import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirmClose } from "../lib/unsaved-changes";
import { useNotebookStore } from "../store/notebookStore";
import { openOrSwitch, replacingBlankTab } from "../lib/file-drop";
import { nbCreate, nbGetState, nbList, nbClose, nbOpen, nbSetActive, getInitialFileArgs, takePendingOpens } from "../lib/notebook-commands";
import type { SyncCell } from "../lib/notebook-commands";
import { openFilePath } from "../lib/notebooks-client";
import { markDirty, cleanup } from "../lib/dirty-inputs";
import type { CellOutput, CellStatus, CellType } from "../types/notebook";
import type { SessionStatus } from "../types/maxima";

/** Returns true when this window is the primary (first) window. */
function isMainWindow(): boolean {
  return getCurrentWindow().label === "main";
}

/** Apply a notebook-state-changed event from the backend to this window's tabs. */
export function handleNotebookStateEvent(payload: NotebookStateEvent) {
  const { notebook_id, cells: syncCells, effect, cell_id, can_undo, can_redo, trusted } = payload;

  // Ensure tab exists (e.g. created by MCP) — only auto-adopt in the main window
  const store = useNotebookStore.getState();
  if (!store.notebooks[notebook_id]) {
    if (isMainWindow()) {
      store.addTab(notebook_id);
    } else {
      return; // Ignore events for notebooks not in this window
    }
  }

  // A run was blocked, from this window or over MCP: show the trust banner.
  // Nothing in the notebook changed.
  if (effect === "trust_required") {
    store.setPendingTrustFunctions(notebook_id, payload.trust_functions ?? []);
    return;
  }

  // Opening gave some cells new IDs: the notebook no longer matches its
  // file, so the next save writes good ones
  if (effect === "ids_replaced") {
    store.markNotebookDirty(notebook_id);
    return;
  }

  // Opened or saved, from this window or over MCP: the tab follows the file.
  // Nothing in the notebook changed.
  if (effect === "path_changed") {
    store.setNotebookPath(notebook_id, payload.path ?? null);
    return;
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

/** Apply a notebook lifecycle event made over MCP to this window's tabs. */
export function handleLifecycleEvent(notebook_id: string, eventType: string) {
  const store = useNotebookStore.getState();

  if (eventType === "created") {
    // Only auto-adopt MCP-created notebooks in the main window
    if (!store.notebooks[notebook_id] && isMainWindow()) {
      store.addTab(notebook_id);
    }
  } else if (eventType === "active_changed") {
    // Follow MCP's switch only in the window that holds the notebook
    if (store.notebooks[notebook_id]) {
      store.setActiveTab(notebook_id);
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
      // Dirty tab — Save, Don't Save or Cancel
      store.setClosePending(notebook_id, true);
      confirmClose([notebook_id]).then((confirmed) => {
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

export interface NotebookStateEvent {
  notebook_id: string;
  effect: string;
  cell_id: string | null;
  cells: SyncCell[];
  can_undo: boolean;
  can_redo: boolean;
  trusted: boolean;
  /** With `trust_required`: the dangerous functions that blocked the run. */
  trust_functions?: string[];
  /** With `path_changed`: the notebook's file path, absent when cleared. */
  path?: string;
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

export function mapSyncCells(syncCells: SyncCell[]) {
  return syncCells.map((sc) => {
    let output: CellOutput | null = null;
    if (sc.output) {
      output = {
        textOutput: sc.output.text_output,
        latex: sc.output.latex,
        plotSvg: sc.output.plot_svg,
        plotData: sc.output.plot_data,
        imagePng: sc.output.image_png,
        error: sc.output.error,
        errorInfo: sc.output.error_info,
        errorTranslated: sc.output.error_translated ?? null,
        notes: sc.output.notes ?? [],
        isError: sc.output.is_error,
        durationMs: sc.output.duration_ms,
        outputLabel: sc.output.output_label,
        executionCount: sc.output.execution_count,
        textPlain: sc.output.text_plain,
        stale: sc.output.stale,
      };
    }
    return {
      id: sc.id,
      cellType: sc.cell_type as CellType,
      input: sc.input,
      output,
      status: (sc.status as CellStatus) ?? "idle",
      silent: sc.silent ?? false,
    };
  });
}

function fileName(path: string): string {
  return path.split("/").pop()?.split("\\").pop() ?? path;
}

/**
 * Open a notebook file in a new tab.  The backend reads the file, loads the
 * cells and records the path; its events fill the tab.  If the file can't be
 * opened, the tab goes again and the error is thrown.
 */
export async function openNotebookTab(path: string): Promise<void> {
  const { notebook_id } = await nbCreate();
  const store = useNotebookStore.getState();
  store.addTab(notebook_id, fileName(path));
  store.setActiveTab(notebook_id);
  await nbSetActive(notebook_id);
  try {
    await nbOpen(path, notebook_id);
  } catch (e) {
    await nbClose(notebook_id).catch(() => {});
    useNotebookStore.getState().removeTab(notebook_id);
    throw e;
  }
}

/**
 * Open a file from a path into the appropriate tab type: a notebook
 * (.macnb/.ipynb) in a backend notebook, a JSON file in a plot tab.
 */
async function openFileAsTab(path: string): Promise<void> {
  const result = await openFilePath(path);
  if (result.type === "plot") {
    const id = `plot-${crypto.randomUUID()}`;
    useNotebookStore.getState().addPlotTab(id, fileName(path), result.plotData, result.path);
  } else {
    await openNotebookTab(path);
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
      // Main window: files from the command line first, then files macOS
      // asked Aximar to open as it started (a double-click, Open With).
      // The queue is drained only past the ignore check, so strict mode's
      // discarded first run can't take them.
      getInitialFileArgs().then(async (args) => {
        if (ignore) return;
        const fileArgs = [...(args ?? []), ...(await takePendingOpens())];

        if (fileArgs.length > 0) {
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
            store.setNotebookPath(nb.id, nb.path);
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
      (event) => handleNotebookStateEvent(event.payload)
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // --- Backend → Frontend: listen for notebook lifecycle events (MCP) ---
  useEffect(() => {
    const unlisten = listen<{ notebook_id: string; event: string }>(
      "notebook-lifecycle",
      (event) => handleLifecycleEvent(event.payload.notebook_id, event.payload.event)
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

  // --- Files macOS asks Aximar to open while it runs (a drop on the Dock
  // icon, Open With, a double-click): the backend queues them and says so ---
  useEffect(() => {
    if (!isMainWindow()) return;
    const openWaiting = async () => {
      const paths = await takePendingOpens();
      if (paths.length === 0) return;
      // A file that launched Aximar can arrive after the window made its
      // blank notebook; the file takes its place
      await replacingBlankTab(async () => {
        for (const path of paths) {
          try {
            await openOrSwitch(path, openFileAsTab);
          } catch (e) {
            console.warn("Failed to open file:", path, e);
          }
        }
      });
    };
    const unlisten = listen("files-waiting", openWaiting);
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
          markDirty(nbId, cell.id, cell.input);
        }
      }
    });

    return () => {
      unsubscribe();
      cleanup();
    };
  }, []);
}
