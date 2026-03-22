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

interface NotebookState {
  cells: Cell[];
  sessionStatus: SessionStatus;
  theme: Theme;
  cellStyle: CellStyle;
  activeCellId: string | null;
  executionCounter: number;
  filePath: string | null;
  isDirty: boolean;

  addCell: (afterId?: string) => void;
  addMarkdownCell: (afterId?: string) => void;
  addCellWithInput: (afterId: string, input: string) => string;
  deleteCell: (id: string) => void;
  moveCell: (id: string, direction: "up" | "down") => void;
  updateCellInput: (id: string, input: string) => void;
  setCellStatus: (id: string, status: CellStatus) => void;
  setCellOutput: (id: string, output: CellOutput) => void;
  setSessionStatus: (status: SessionStatus) => void;
  setTheme: (theme: Theme) => void;
  setCellStyle: (style: CellStyle) => void;
  setActiveCellId: (id: string | null) => void;
  insertTextInActiveCell: (text: string) => void;
  toggleCellType: (id: string) => void;
  loadNotebook: (cells: NotebookCell[], filePath?: string | null) => void;
  newNotebook: () => void;
  setFilePath: (path: string | null) => void;
  markClean: () => void;
}

export const useNotebookStore = create<NotebookState>((set) => ({
  cells: [createCell()],
  sessionStatus: "Stopped",
  theme: "auto",
  cellStyle: "card",
  activeCellId: null,
  executionCounter: 0,
  filePath: null,
  isDirty: false,

  addCell: (afterId?: string) =>
    set((state) => {
      const newCell = createCell();
      if (!afterId) {
        return { cells: [...state.cells, newCell], isDirty: true };
      }
      const index = state.cells.findIndex((c) => c.id === afterId);
      if (index === -1) {
        return { cells: [...state.cells, newCell], isDirty: true };
      }
      const cells = [...state.cells];
      cells.splice(index + 1, 0, newCell);
      return { cells, isDirty: true };
    }),

  addMarkdownCell: (afterId?: string) =>
    set((state) => {
      const newCell = createCell("markdown");
      if (!afterId) {
        return { cells: [...state.cells, newCell], isDirty: true };
      }
      const index = state.cells.findIndex((c) => c.id === afterId);
      if (index === -1) {
        return { cells: [...state.cells, newCell], isDirty: true };
      }
      const cells = [...state.cells];
      cells.splice(index + 1, 0, newCell);
      return { cells, isDirty: true };
    }),

  addCellWithInput: (afterId: string, input: string) => {
    const newCell: Cell = { ...createCell(), input };
    set((state) => {
      const index = state.cells.findIndex((c) => c.id === afterId);
      if (index === -1) {
        return { cells: [...state.cells, newCell], isDirty: true };
      }
      const cells = [...state.cells];
      cells.splice(index + 1, 0, newCell);
      return { cells, isDirty: true };
    });
    return newCell.id;
  },

  deleteCell: (id: string) =>
    set((state) => {
      if (state.cells.length <= 1) return state;
      return { cells: state.cells.filter((c) => c.id !== id), isDirty: true };
    }),

  moveCell: (id: string, direction: "up" | "down") =>
    set((state) => {
      const index = state.cells.findIndex((c) => c.id === id);
      if (index === -1) return state;
      const target = direction === "up" ? index - 1 : index + 1;
      if (target < 0 || target >= state.cells.length) return state;
      const cells = [...state.cells];
      [cells[index], cells[target]] = [cells[target], cells[index]];
      return { cells, isDirty: true };
    }),

  updateCellInput: (id: string, input: string) =>
    set((state) => ({
      cells: state.cells.map((c) => (c.id === id ? { ...c, input } : c)),
      isDirty: true,
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

  setCellStyle: (cellStyle: CellStyle) => set({ cellStyle }),

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
        isDirty: true,
      };
    }),

  toggleCellType: (id: string) =>
    set((state) => ({
      cells: state.cells.map((c) =>
        c.id === id
          ? { ...c, cellType: c.cellType === "code" ? "markdown" : "code", output: null, status: "idle" }
          : c
      ),
      isDirty: true,
    })),

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
    })),

  newNotebook: () =>
    set(() => ({
      cells: [createCell()],
      executionCounter: 0,
      filePath: null,
      isDirty: false,
    })),

  setFilePath: (path: string | null) => set({ filePath: path }),

  markClean: () => set({ isDirty: false }),
}));
