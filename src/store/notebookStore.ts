import { create } from "zustand";
import { nanoid } from "nanoid";
import type { Cell, CellOutput, CellStatus } from "../types/notebook";
import type { SessionStatus } from "../types/maxima";
import type { NotebookCell } from "../types/notebooks";
import { cellSourceText } from "../types/notebooks";

function createCell(): Cell {
  return {
    id: nanoid(),
    input: "",
    output: null,
    status: "idle",
  };
}

export type Theme = "auto" | "light" | "dark";

interface NotebookState {
  cells: Cell[];
  sessionStatus: SessionStatus;
  theme: Theme;
  activeCellId: string | null;
  executionCounter: number;

  addCell: (afterId?: string) => void;
  addCellWithInput: (afterId: string, input: string) => string;
  deleteCell: (id: string) => void;
  updateCellInput: (id: string, input: string) => void;
  setCellStatus: (id: string, status: CellStatus) => void;
  setCellOutput: (id: string, output: CellOutput) => void;
  setSessionStatus: (status: SessionStatus) => void;
  setTheme: (theme: Theme) => void;
  setActiveCellId: (id: string | null) => void;
  insertTextInActiveCell: (text: string) => void;
  loadNotebook: (cells: NotebookCell[]) => void;
}

export const useNotebookStore = create<NotebookState>((set) => ({
  cells: [createCell()],
  sessionStatus: "Stopped",
  theme: "auto",
  activeCellId: null,
  executionCounter: 0,

  addCell: (afterId?: string) =>
    set((state) => {
      const newCell = createCell();
      if (!afterId) {
        return { cells: [...state.cells, newCell] };
      }
      const index = state.cells.findIndex((c) => c.id === afterId);
      if (index === -1) {
        return { cells: [...state.cells, newCell] };
      }
      const cells = [...state.cells];
      cells.splice(index + 1, 0, newCell);
      return { cells };
    }),

  addCellWithInput: (afterId: string, input: string) => {
    const newCell: Cell = { ...createCell(), input };
    set((state) => {
      const index = state.cells.findIndex((c) => c.id === afterId);
      if (index === -1) {
        return { cells: [...state.cells, newCell] };
      }
      const cells = [...state.cells];
      cells.splice(index + 1, 0, newCell);
      return { cells };
    });
    return newCell.id;
  },

  deleteCell: (id: string) =>
    set((state) => {
      if (state.cells.length <= 1) return state;
      return { cells: state.cells.filter((c) => c.id !== id) };
    }),

  updateCellInput: (id: string, input: string) =>
    set((state) => ({
      cells: state.cells.map((c) => (c.id === id ? { ...c, input } : c)),
    })),

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

  setActiveCellId: (id: string | null) => set({ activeCellId: id }),

  insertTextInActiveCell: (text: string) =>
    set((state) => {
      if (!state.activeCellId) return state;
      return {
        cells: state.cells.map((c) =>
          c.id === state.activeCellId
            ? { ...c, input: c.input + text }
            : c
        ),
      };
    }),

  loadNotebook: (newCells: NotebookCell[]) =>
    set(() => ({
      executionCounter: 0,
      cells: newCells
        .filter((c) => c.cell_type === "code")
        .map((c) => ({
          ...createCell(),
          input: cellSourceText(c.source),
        })),
    })),
}));
