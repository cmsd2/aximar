import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useNotebookStore } from "../store/notebookStore";
import type { Cell, CellOutput, CellStatus, CellType } from "../types/notebook";

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

interface NotebookSyncPayload {
  cells: SyncCell[];
}

const SYNC_DEBOUNCE_MS = 200;

/**
 * Hook that keeps the frontend notebook state and the backend McpNotebook
 * in sync for connected-mode MCP.
 *
 * - Debounced push: after each cell mutation, sync the frontend state to the
 *   backend so MCP reads up-to-date content.
 * - MCP event listener: when MCP modifies the notebook, the backend emits
 *   "mcp-notebook-sync" and we update the frontend Zustand store.
 */
export function useMcpSync() {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Track whether we're currently applying an MCP update to avoid echo loops
  const applyingMcpUpdate = useRef(false);

  // --- Frontend → Backend sync (debounced) ---
  useEffect(() => {
    const unsubscribe = useNotebookStore.subscribe((state, prevState) => {
      // Skip sync if we're applying an MCP-triggered update
      if (applyingMcpUpdate.current) return;
      // Only sync when cells actually change
      if (state.cells === prevState.cells) return;

      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => {
        const syncCells: SyncCell[] = state.cells.map((c) => ({
          id: c.id,
          cell_type: c.cellType,
          input: c.input,
        }));
        invoke("sync_notebook_state", { cells: syncCells }).catch((e) =>
          console.warn("MCP sync failed:", e)
        );
      }, SYNC_DEBOUNCE_MS);
    });

    return () => {
      unsubscribe();
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  // --- Backend → Frontend sync (MCP events) ---
  useEffect(() => {
    const unlisten = listen<NotebookSyncPayload>(
      "mcp-notebook-sync",
      (event) => {
        const { cells: syncCells } = event.payload;
        applyingMcpUpdate.current = true;

        useNotebookStore.setState((state) => {
          const newCells: Cell[] = syncCells.map((sc) => {
            const existing = state.cells.find((c) => c.id === sc.id);
            // Use incoming output from MCP if present, otherwise preserve existing
            let output: CellOutput | null = existing?.output ?? null;
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
              status: (sc.status as CellStatus) ?? existing?.status ?? "idle",
            };
          });
          return { cells: newCells, isDirty: true };
        });

        // Reset the flag after a microtask so the subscribe callback sees it
        Promise.resolve().then(() => {
          applyingMcpUpdate.current = false;
        });
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
