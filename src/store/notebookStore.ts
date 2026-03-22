import { create } from "zustand";
import { nanoid } from "nanoid";
import type { Cell, CellOutput, CellStatus, CellType } from "../types/notebook";
import type { SessionStatus } from "../types/maxima";
import type { NotebookCell } from "../types/notebooks";
import { cellSourceText } from "../types/notebooks";

function createCell(cellType: CellType = "code"): Cell {
  return {
    id: nanoid(),
    cellType,
    input: "",
    output: null,
    status: "idle",
  };
}

export type Theme = "auto" | "light" | "dark";
export type CellStyle = "card" | "bracket";
export type AutocompleteMode = "hint" | "snippet" | "active-hint";

type UndoableSnapshot = { cells: Cell[] };

const MAX_UNDO = 50;
const INPUT_DEBOUNCE_MS = 500;

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

  // Undo/redo
  _undoPast: UndoableSnapshot[];
  _undoFuture: UndoableSnapshot[];
  _lastInputSnapshotTime: number;
  _forceNextInputSnapshot: boolean;

  addCell: (afterId?: string) => string;
  addMarkdownCell: (afterId?: string) => string;
  addCellWithInput: (afterId: string, input: string) => string;
  deleteCell: (id: string) => void;
  moveCell: (id: string, direction: "up" | "down") => void;
  updateCellInput: (id: string, input: string) => void;
  setCellStatus: (id: string, status: CellStatus) => void;
  setCellOutput: (id: string, output: CellOutput) => void;
  setSessionStatus: (status: SessionStatus) => void;
  setTheme: (theme: Theme) => void;
  setCellStyle: (style: CellStyle) => void;
  setAutocompleteMode: (mode: AutocompleteMode) => void;
  setActiveCellId: (id: string | null) => void;
  insertTextInActiveCell: (text: string) => void;
  toggleCellType: (id: string) => void;
  loadNotebook: (cells: NotebookCell[], filePath?: string | null) => void;
  newNotebook: () => void;
  setFilePath: (path: string | null) => void;
  markClean: () => void;
  undo: () => void;
  redo: () => void;
  forceInputSnapshot: () => void;
}

function snapshotCells(cells: Cell[]): Cell[] {
  return structuredClone(cells);
}

function pushSnapshot(past: UndoableSnapshot[], cells: Cell[]): UndoableSnapshot[] {
  const next = [...past, { cells: snapshotCells(cells) }];
  if (next.length > MAX_UNDO) next.shift();
  return next;
}

export const useNotebookStore = create<NotebookState>((set) => ({
  cells: [createCell()],
  sessionStatus: "Stopped",
  theme: "auto",
  cellStyle: "card",
  autocompleteMode: "active-hint",
  activeCellId: null,
  executionCounter: 0,
  filePath: null,
  isDirty: false,

  _undoPast: [],
  _undoFuture: [],
  _lastInputSnapshotTime: 0,
  _forceNextInputSnapshot: false,

  addCell: (afterId?: string) => {
    const newCell = createCell();
    set((state) => {
      const past = pushSnapshot(state._undoPast, state.cells);
      if (!afterId) {
        return { cells: [...state.cells, newCell], isDirty: true, _undoPast: past, _undoFuture: [] };
      }
      const index = state.cells.findIndex((c) => c.id === afterId);
      if (index === -1) {
        return { cells: [...state.cells, newCell], isDirty: true, _undoPast: past, _undoFuture: [] };
      }
      const cells = [...state.cells];
      cells.splice(index + 1, 0, newCell);
      return { cells, isDirty: true, _undoPast: past, _undoFuture: [] };
    });
    return newCell.id;
  },

  addMarkdownCell: (afterId?: string) => {
    const newCell = createCell("markdown");
    set((state) => {
      const past = pushSnapshot(state._undoPast, state.cells);
      if (!afterId) {
        return { cells: [...state.cells, newCell], isDirty: true, _undoPast: past, _undoFuture: [] };
      }
      const index = state.cells.findIndex((c) => c.id === afterId);
      if (index === -1) {
        return { cells: [...state.cells, newCell], isDirty: true, _undoPast: past, _undoFuture: [] };
      }
      const cells = [...state.cells];
      cells.splice(index + 1, 0, newCell);
      return { cells, isDirty: true, _undoPast: past, _undoFuture: [] };
    });
    return newCell.id;
  },

  addCellWithInput: (afterId: string, input: string) => {
    const newCell: Cell = { ...createCell(), input };
    set((state) => {
      const past = pushSnapshot(state._undoPast, state.cells);
      const index = state.cells.findIndex((c) => c.id === afterId);
      if (index === -1) {
        return { cells: [...state.cells, newCell], isDirty: true, _undoPast: past, _undoFuture: [] };
      }
      const cells = [...state.cells];
      cells.splice(index + 1, 0, newCell);
      return { cells, isDirty: true, _undoPast: past, _undoFuture: [] };
    });
    return newCell.id;
  },

  deleteCell: (id: string) =>
    set((state) => {
      if (state.cells.length <= 1) return state;
      const past = pushSnapshot(state._undoPast, state.cells);
      return { cells: state.cells.filter((c) => c.id !== id), isDirty: true, _undoPast: past, _undoFuture: [] };
    }),

  moveCell: (id: string, direction: "up" | "down") =>
    set((state) => {
      const index = state.cells.findIndex((c) => c.id === id);
      if (index === -1) return state;
      const target = direction === "up" ? index - 1 : index + 1;
      if (target < 0 || target >= state.cells.length) return state;
      const past = pushSnapshot(state._undoPast, state.cells);
      const cells = [...state.cells];
      [cells[index], cells[target]] = [cells[target], cells[index]];
      return { cells, isDirty: true, _undoPast: past, _undoFuture: [] };
    }),

  updateCellInput: (id: string, input: string) =>
    set((state) => {
      const now = Date.now();
      const elapsed = now - state._lastInputSnapshotTime;
      const shouldSnapshot = state._forceNextInputSnapshot || elapsed >= INPUT_DEBOUNCE_MS;

      let past = state._undoPast;
      let lastTime = state._lastInputSnapshotTime;
      let force = state._forceNextInputSnapshot;

      if (shouldSnapshot) {
        past = pushSnapshot(past, state.cells);
        lastTime = now;
        force = false;
      }

      return {
        cells: state.cells.map((c) => (c.id === id ? { ...c, input } : c)),
        isDirty: true,
        _undoPast: past,
        _undoFuture: [],
        _lastInputSnapshotTime: lastTime,
        _forceNextInputSnapshot: force,
      };
    }),

  setCellStatus: (id: string, status: CellStatus) =>
    set((state) => ({
      cells: state.cells.map((c) => (c.id === id ? { ...c, status } : c)),
    })),

  setCellOutput: (id: string, output: CellOutput) =>
    set((state) => {
      const nextCount = state.executionCounter + 1;
      const stamped = { ...output, executionCount: nextCount };
      return {
        executionCounter: nextCount,
        cells: state.cells.map((c) =>
          c.id === id ? { ...c, output: stamped, status: output.isError ? "error" : "success" } : c
        ),
      };
    }),

  setSessionStatus: (status: SessionStatus) => set({ sessionStatus: status }),

  setTheme: (theme: Theme) => set({ theme }),

  setCellStyle: (cellStyle: CellStyle) => set({ cellStyle }),

  setAutocompleteMode: (autocompleteMode: AutocompleteMode) => set({ autocompleteMode }),

  setActiveCellId: (id: string | null) => set({ activeCellId: id }),

  insertTextInActiveCell: (text: string) =>
    set((state) => {
      if (!state.activeCellId) return state;
      const past = pushSnapshot(state._undoPast, state.cells);
      return {
        cells: state.cells.map((c) =>
          c.id === state.activeCellId
            ? { ...c, input: c.input + text }
            : c
        ),
        isDirty: true,
        _undoPast: past,
        _undoFuture: [],
      };
    }),

  toggleCellType: (id: string) =>
    set((state) => {
      const past = pushSnapshot(state._undoPast, state.cells);
      return {
        cells: state.cells.map((c) =>
          c.id === id
            ? { ...c, cellType: c.cellType === "code" ? "markdown" : "code", output: null, status: "idle" }
            : c
        ),
        isDirty: true,
        _undoPast: past,
        _undoFuture: [],
      };
    }),

  loadNotebook: (newCells: NotebookCell[], filePath?: string | null) =>
    set(() => ({
      executionCounter: 0,
      filePath: filePath ?? null,
      isDirty: false,
      cells: newCells
        .filter((c) => c.cell_type !== "raw")
        .map((c) => ({
          ...createCell(c.cell_type === "markdown" ? "markdown" : "code"),
          input: cellSourceText(c.source),
        })),
      _undoPast: [],
      _undoFuture: [],
      _lastInputSnapshotTime: 0,
      _forceNextInputSnapshot: false,
    })),

  newNotebook: () =>
    set(() => ({
      cells: [createCell()],
      executionCounter: 0,
      filePath: null,
      isDirty: false,
      _undoPast: [],
      _undoFuture: [],
      _lastInputSnapshotTime: 0,
      _forceNextInputSnapshot: false,
    })),

  setFilePath: (path: string | null) => set({ filePath: path }),

  markClean: () => set({ isDirty: false }),

  undo: () =>
    set((state) => {
      if (state._undoPast.length === 0) return state;
      const past = [...state._undoPast];
      const snapshot = past.pop()!;
      const future = [{ cells: snapshotCells(state.cells) }, ...state._undoFuture];
      return {
        cells: snapshot.cells,
        _undoPast: past,
        _undoFuture: future,
        isDirty: true,
      };
    }),

  redo: () =>
    set((state) => {
      if (state._undoFuture.length === 0) return state;
      const future = [...state._undoFuture];
      const snapshot = future.shift()!;
      const past = [...state._undoPast, { cells: snapshotCells(state.cells) }];
      return {
        cells: snapshot.cells,
        _undoPast: past,
        _undoFuture: future,
        isDirty: true,
      };
    }),

  forceInputSnapshot: () => set({ _forceNextInputSnapshot: true }),
}));
