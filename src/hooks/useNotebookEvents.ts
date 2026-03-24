import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useNotebookStore } from "../store/notebookStore";
import { nbGetState, nbUpdateCellInput } from "../lib/notebook-commands";
import type { CellOutput, CellStatus, CellType } from "../types/notebook";

interface SyncCellOutput {
  text_output: string;
  latex: string | null;
  plot_svg: string | null;
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
  effect: string;
  cell_id: string | null;
  cells: SyncCell[];
  can_undo: boolean;
  can_redo: boolean;
}

const INPUT_SYNC_DEBOUNCE_MS = 300;

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

/**
 * Hook that listens for `notebook-state-changed` events from the backend
 * and applies them to the frontend Zustand store. Also handles debounced
 * input sync from local edits to the backend.
 */
export function useNotebookEvents() {
  const inputTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Track which cell inputs are "dirty" locally so we can debounce sync
  const dirtyInputsRef = useRef<Map<string, string>>(new Map());

  // --- Backend → Frontend: listen for state changes ---
  useEffect(() => {
    const unlisten = listen<NotebookStateEvent>(
      "notebook-state-changed",
      (event) => {
        const { cells: syncCells, effect, cell_id, can_undo, can_redo } =
          event.payload;
        useNotebookStore.getState().applyBackendState(
          syncCells.map((sc) => {
            let output: CellOutput | null = null;
            if (sc.output) {
              output = {
                textOutput: sc.output.text_output,
                latex: sc.output.latex,
                plotSvg: sc.output.plot_svg,
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
          }),
          effect,
          cell_id ?? undefined,
          can_undo,
          can_redo
        );

        // Focus the newly added cell after React renders it
        if (effect === "cell_added" && cell_id) {
          requestAnimationFrame(() => focusCellEditor(cell_id));
        }
      }
    );

    // Fetch initial state from the backend so the frontend starts in sync
    nbGetState().then((state) => {
      const { cells: syncCells, effect, cell_id, can_undo, can_redo } = state;
      useNotebookStore.getState().applyBackendState(
        syncCells.map((sc) => ({
          id: sc.id,
          cellType: sc.cell_type as CellType,
          input: sc.input,
          output: null,
          status: (sc.status as CellStatus) ?? "idle",
        })),
        effect,
        cell_id ?? undefined,
        can_undo,
        can_redo
      );
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // --- Frontend → Backend: debounced input sync ---
  useEffect(() => {
    const unsubscribe = useNotebookStore.subscribe((state, prevState) => {
      // Find cells with changed input
      for (const cell of state.cells) {
        const prev = prevState.cells.find((c) => c.id === cell.id);
        if (prev && prev.input !== cell.input) {
          dirtyInputsRef.current.set(cell.id, cell.input);
        }
      }

      if (dirtyInputsRef.current.size > 0) {
        if (inputTimerRef.current) clearTimeout(inputTimerRef.current);
        inputTimerRef.current = setTimeout(() => {
          const dirty = dirtyInputsRef.current;
          dirtyInputsRef.current = new Map();
          for (const [cellId, input] of dirty) {
            nbUpdateCellInput(cellId, input).catch((e) =>
              console.warn("Input sync failed:", e)
            );
          }
        }, INPUT_SYNC_DEBOUNCE_MS);
      }
    });

    return () => {
      unsubscribe();
      if (inputTimerRef.current) clearTimeout(inputTimerRef.current);
    };
  }, []);
}
