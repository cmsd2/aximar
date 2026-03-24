import { create } from "zustand";
import type { Cell } from "../types/notebook";
import type { SessionStatus } from "../types/maxima";

export type Theme = "auto" | "light" | "dark";
export type CellStyle = "card" | "bracket";
export type AutocompleteMode = "hint" | "snippet" | "active-hint";

interface NotebookState {
  cells: Cell[];
  sessionStatus: SessionStatus;
  theme: Theme;
  cellStyle: CellStyle;
  autocompleteMode: AutocompleteMode;
  activeCellId: string | null;
  executionCounter: number;
  filePath: string | null;
  isDirty: boolean;
  pendingCursorMove: { cellId: string; pos: number } | null;

  // Backend undo/redo state (read-only for UI)
  canUndo: boolean;
  canRedo: boolean;

  // Local-first cell input editing (not sent to backend immediately)
  updateCellInput: (id: string, input: string) => void;
  // Insert text into the active cell (local-first, for command palette)
  insertTextInActiveCell: (text: string) => void;

  // UI-only state setters
  setSessionStatus: (status: SessionStatus) => void;
  setTheme: (theme: Theme) => void;
  setCellStyle: (style: CellStyle) => void;
  setAutocompleteMode: (mode: AutocompleteMode) => void;
  setActiveCellId: (id: string | null) => void;
  setFilePath: (path: string | null) => void;
  markClean: () => void;
  setPendingCursorMove: (move: { cellId: string; pos: number }) => void;
  clearPendingCursorMove: () => void;

  // Apply full state from backend events
  applyBackendState: (
    cells: Cell[],
    effect: string,
    cellId?: string,
    canUndo?: boolean,
    canRedo?: boolean
  ) => void;
}

export const useNotebookStore = create<NotebookState>((set) => ({
  cells: [],
  sessionStatus: "Stopped",
  theme: "auto",
  cellStyle: "card",
  autocompleteMode: "active-hint",
  activeCellId: null,
  executionCounter: 0,
  filePath: null,
  isDirty: false,
  pendingCursorMove: null,
  canUndo: false,
  canRedo: false,

  updateCellInput: (id: string, input: string) =>
    set((state) => ({
      cells: state.cells.map((c) => (c.id === id ? { ...c, input } : c)),
      isDirty: true,
    })),

  insertTextInActiveCell: (text: string) =>
    set((state) => {
      if (!state.activeCellId) return state;
      return {
        cells: state.cells.map((c) =>
          c.id === state.activeCellId ? { ...c, input: c.input + text } : c
        ),
        isDirty: true,
      };
    }),

  setSessionStatus: (status: SessionStatus) => set({ sessionStatus: status }),
  setTheme: (theme: Theme) => set({ theme }),
  setCellStyle: (cellStyle: CellStyle) => set({ cellStyle }),
  setAutocompleteMode: (autocompleteMode: AutocompleteMode) =>
    set({ autocompleteMode }),
  setActiveCellId: (id: string | null) => set({ activeCellId: id }),
  setFilePath: (path: string | null) => set({ filePath: path }),
  markClean: () => set({ isDirty: false }),
  setPendingCursorMove: (move: { cellId: string; pos: number }) =>
    set({ pendingCursorMove: move }),
  clearPendingCursorMove: () => set({ pendingCursorMove: null }),

  applyBackendState: (
    cells: Cell[],
    effect: string,
    cellId?: string,
    canUndo?: boolean,
    canRedo?: boolean
  ) =>
    set((state) => {
      // Merge backend cells with local state, preserving local input edits
      // that may not have been synced yet.
      const isReplace = effect === "notebook_replaced";
      const mergedCells = cells.map((backendCell) => {
        const localCell = state.cells.find((c) => c.id === backendCell.id);
        // For input updates, notebook replacements (MCP changes, undo/redo,
        // new/load), or new cells: use backend input. Otherwise preserve
        // local input which may have pending debounced sync.
        const isInputEffect =
          effect === "cell_input_updated" && cellId === backendCell.id;
        const useBackendInput =
          isInputEffect || isReplace || !localCell;
        const input = useBackendInput
          ? backendCell.input
          : localCell.input;
        return {
          ...backendCell,
          input,
        };
      });

      // Determine execution counter from the highest execution count in cells
      let maxExecCount = state.executionCounter;
      for (const cell of mergedCells) {
        if (cell.output?.executionCount != null) {
          maxExecCount = Math.max(maxExecCount, cell.output.executionCount);
        }
      }

      // Auto-activate newly added cells; on notebook replace, activate the first cell
      let newActiveCellId = state.activeCellId;
      if (effect === "cell_added" && cellId) {
        newActiveCellId = cellId;
      } else if (isReplace && mergedCells.length > 0) {
        newActiveCellId = mergedCells[0].id;
      }

      return {
        cells: mergedCells,
        activeCellId: newActiveCellId,
        executionCounter: maxExecCount,
        isDirty: isReplace ? false : true,
        canUndo: canUndo ?? state.canUndo,
        canRedo: canRedo ?? state.canRedo,
        // Clear filePath on new notebook
        ...(effect === "notebook_replaced" && !cellId
          ? { filePath: null }
          : {}),
      };
    }),
}));
