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

function buildSyncCells(cells: Cell[]): SyncCell[] {
  return cells.map((c) => ({
    id: c.id,
    cell_type: c.cellType,
    input: c.input,
  }));
}

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

  // Version counter to detect echo loops: incremented each time we apply
  // an MCP event, checked by the debounced subscriber to skip echoed syncs.
  const mcpVersionRef = useRef(0);

  // Whether the initial sync has been sent to the backend.
  const readyRef = useRef(false);

  // --- Initial sync on mount ---
  useEffect(() => {
    const syncCells = buildSyncCells(useNotebookStore.getState().cells);
    invoke("sync_notebook_state", { cells: syncCells })
      .then(() => {
        readyRef.current = true;
      })
      .catch((e) => {
        console.warn("Initial MCP sync failed:", e);
        // Mark ready anyway so the app doesn't stall
        readyRef.current = true;
      });
  }, []);

  // --- Frontend → Backend sync (debounced) ---
  useEffect(() => {
    const unsubscribe = useNotebookStore.subscribe((state, prevState) => {
      // Only sync when cells actually change
      if (state.cells === prevState.cells) return;

      // Capture the MCP version at the time the change was detected
      const versionAtChange = mcpVersionRef.current;

      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => {
        // If an MCP update arrived between scheduling and firing, skip
        // this sync — the change was triggered by an MCP event.
        if (mcpVersionRef.current !== versionAtChange) return;

        const syncCells = buildSyncCells(
          useNotebookStore.getState().cells
        );
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
        // Don't apply MCP events until initial sync is complete
        if (!readyRef.current) return;

        const { cells: syncCells } = event.payload;

        // Bump version before setState so the subscriber sees the new value
        mcpVersionRef.current += 1;

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
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
