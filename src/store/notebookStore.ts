import { create } from "zustand";
import { nanoid } from "nanoid";
import type { Cell, CellOutput, CellStatus } from "../types/notebook";
import type { SessionStatus } from "../types/maxima";

function createCell(): Cell {
  return {
    id: nanoid(),
    input: "",
    output: null,
    status: "idle",
  };
}

interface NotebookState {
  cells: Cell[];
  sessionStatus: SessionStatus;

  addCell: (afterId?: string) => void;
  deleteCell: (id: string) => void;
  updateCellInput: (id: string, input: string) => void;
  setCellStatus: (id: string, status: CellStatus) => void;
  setCellOutput: (id: string, output: CellOutput) => void;
  setSessionStatus: (status: SessionStatus) => void;
}

export const useNotebookStore = create<NotebookState>((set) => ({
  cells: [createCell()],
  sessionStatus: "Stopped",

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
    set((state) => ({
      cells: state.cells.map((c) =>
        c.id === id ? { ...c, output, status: output.isError ? "error" : "success" } : c
      ),
    })),

  setSessionStatus: (status: SessionStatus) => set({ sessionStatus: status }),
}));
