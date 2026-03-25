import { create } from "zustand";
import type { Cell } from "../types/notebook";
import type { SessionStatus } from "../types/maxima";

export type Theme = "auto" | "light" | "dark";
export type CellStyle = "card" | "bracket";
export type AutocompleteMode = "hint" | "snippet" | "active-hint";

export interface NotebookTab {
  id: string;
  title: string;
  cells: Cell[];
  sessionStatus: SessionStatus;
  activeCellId: string | null;
  executionCounter: number;
  filePath: string | null;
  isDirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
  closePending: boolean;
  pendingCursorMove: { cellId: string; pos: number } | null;
}

function createDefaultTab(id: string): NotebookTab {
  return {
    id,
    title: "Untitled",
    cells: [],
    sessionStatus: "Stopped",
    activeCellId: null,
    executionCounter: 0,
    filePath: null,
    isDirty: false,
    canUndo: false,
    canRedo: false,
    closePending: false,
    pendingCursorMove: null,
  };
}

interface NotebookState {
  notebooks: Record<string, NotebookTab>;
  activeNotebookId: string | null;

  // Global settings (not per-notebook)
  theme: Theme;
  cellStyle: CellStyle;
  autocompleteMode: AutocompleteMode;

  // --- Tab lifecycle ---
  addTab: (id: string, title?: string) => void;
  removeTab: (id: string) => void;
  setActiveTab: (id: string) => void;

  // --- Per-notebook state derived from active tab (backward-compatible) ---

  // Local-first cell input editing (not sent to backend immediately)
  updateCellInput: (id: string, input: string) => void;
  // Insert text into the active cell (local-first, for command palette)
  insertTextInActiveCell: (text: string) => void;

  // UI-only state setters
  setSessionStatus: (status: SessionStatus) => void;
  setSessionStatusForNotebook: (notebookId: string, status: SessionStatus) => void;
  setTheme: (theme: Theme) => void;
  setCellStyle: (style: CellStyle) => void;
  setAutocompleteMode: (mode: AutocompleteMode) => void;
  setActiveCellId: (id: string | null) => void;
  setFilePath: (path: string | null) => void;
  setClosePending: (notebookId: string, pending: boolean) => void;
  markClean: () => void;
  setPendingCursorMove: (move: { cellId: string; pos: number }) => void;
  clearPendingCursorMove: () => void;

  // Apply full state from backend events
  applyBackendState: (
    notebookId: string,
    cells: Cell[],
    effect: string,
    cellId?: string,
    canUndo?: boolean,
    canRedo?: boolean
  ) => void;
}

/** Update a single notebook tab within the state, returning updated notebooks map. */
function updateTab(
  notebooks: Record<string, NotebookTab>,
  id: string,
  updater: (tab: NotebookTab) => Partial<NotebookTab>
): Record<string, NotebookTab> {
  const tab = notebooks[id];
  if (!tab) return notebooks;
  return { ...notebooks, [id]: { ...tab, ...updater(tab) } };
}

export const useNotebookStore = create<NotebookState>((set) => ({
  notebooks: {},
  activeNotebookId: null,

  theme: "auto",
  cellStyle: "card",
  autocompleteMode: "active-hint",

  // --- Tab lifecycle ---

  addTab: (id: string, title?: string) =>
    set((state) => {
      const tab = createDefaultTab(id);
      if (title) tab.title = title;
      const notebooks = { ...state.notebooks, [id]: tab };
      return {
        notebooks,
        activeNotebookId: state.activeNotebookId ?? id,
      };
    }),

  removeTab: (id: string) =>
    set((state) => {
      const { [id]: _, ...rest } = state.notebooks;
      const ids = Object.keys(rest);
      let activeNotebookId = state.activeNotebookId;
      if (activeNotebookId === id) {
        activeNotebookId = ids[0] ?? null;
      }
      return { notebooks: rest, activeNotebookId };
    }),

  setActiveTab: (id: string) =>
    set({ activeNotebookId: id }),

  // --- Active-tab mutations (backward-compatible) ---

  updateCellInput: (cellId: string, input: string) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateTab(state.notebooks, nbId, (tab) => ({
          cells: tab.cells.map((c) =>
            c.id === cellId ? { ...c, input } : c
          ),
          isDirty: true,
        })),
      };
    }),

  insertTextInActiveCell: (text: string) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      const tab = state.notebooks[nbId];
      if (!tab || !tab.activeCellId) return state;
      return {
        notebooks: updateTab(state.notebooks, nbId, (tab) => ({
          cells: tab.cells.map((c) =>
            c.id === tab.activeCellId ? { ...c, input: c.input + text } : c
          ),
          isDirty: true,
        })),
      };
    }),

  setSessionStatus: (status: SessionStatus) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateTab(state.notebooks, nbId, () => ({
          sessionStatus: status,
        })),
      };
    }),

  setSessionStatusForNotebook: (notebookId: string, status: SessionStatus) =>
    set((state) => ({
      notebooks: updateTab(state.notebooks, notebookId, () => ({
        sessionStatus: status,
      })),
    })),

  setTheme: (theme: Theme) => set({ theme }),
  setCellStyle: (cellStyle: CellStyle) => set({ cellStyle }),
  setAutocompleteMode: (autocompleteMode: AutocompleteMode) =>
    set({ autocompleteMode }),

  setActiveCellId: (id: string | null) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateTab(state.notebooks, nbId, () => ({
          activeCellId: id,
        })),
      };
    }),

  setFilePath: (path: string | null) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      const title = path
        ? (path.split("/").pop()?.split("\\").pop() ?? "Untitled")
        : "Untitled";
      return {
        notebooks: updateTab(state.notebooks, nbId, () => ({
          filePath: path,
          title,
        })),
      };
    }),

  setClosePending: (notebookId: string, pending: boolean) =>
    set((state) => ({
      notebooks: updateTab(state.notebooks, notebookId, () => ({
        closePending: pending,
      })),
    })),

  markClean: () =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateTab(state.notebooks, nbId, () => ({
          isDirty: false,
        })),
      };
    }),

  setPendingCursorMove: (move: { cellId: string; pos: number }) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateTab(state.notebooks, nbId, () => ({
          pendingCursorMove: move,
        })),
      };
    }),

  clearPendingCursorMove: () =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateTab(state.notebooks, nbId, () => ({
          pendingCursorMove: null,
        })),
      };
    }),

  applyBackendState: (
    notebookId: string,
    cells: Cell[],
    effect: string,
    cellId?: string,
    canUndo?: boolean,
    canRedo?: boolean
  ) =>
    set((state) => {
      const tab = state.notebooks[notebookId];
      if (!tab) return state;

      // Merge backend cells with local state, preserving local input edits
      const isReplace = effect === "notebook_replaced";
      const mergedCells = cells.map((backendCell) => {
        const localCell = tab.cells.find((c) => c.id === backendCell.id);
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
      let maxExecCount = tab.executionCounter;
      for (const cell of mergedCells) {
        if (cell.output?.executionCount != null) {
          maxExecCount = Math.max(maxExecCount, cell.output.executionCount);
        }
      }

      // Auto-activate newly added cells; on notebook replace, activate the first cell
      let newActiveCellId = tab.activeCellId;
      if (effect === "cell_added" && cellId) {
        newActiveCellId = cellId;
      } else if (isReplace && mergedCells.length > 0) {
        newActiveCellId = mergedCells[0].id;
      }

      const updatedTab: NotebookTab = {
        ...tab,
        cells: mergedCells,
        activeCellId: newActiveCellId,
        executionCounter: maxExecCount,
        isDirty: isReplace ? false : true,
        canUndo: canUndo ?? tab.canUndo,
        canRedo: canRedo ?? tab.canRedo,
        // Clear filePath on new notebook (not on load)
        ...(effect === "notebook_replaced" && !cellId
          ? { filePath: null, title: "Untitled" }
          : {}),
      };

      return {
        notebooks: { ...state.notebooks, [notebookId]: updatedTab },
      };
    }),
}));

// ── Selectors ──────────────────────────────────────────────────────

/** Stable fallback so selectors don't create new objects on every call. */
const EMPTY_TAB: NotebookTab = Object.freeze(createDefaultTab("")) as NotebookTab;

/** Return the active notebook tab, or a stable empty tab. */
export function useActiveTab(): NotebookTab {
  return useNotebookStore((s) => {
    const id = s.activeNotebookId;
    if (!id) return EMPTY_TAB;
    return s.notebooks[id] ?? EMPTY_TAB;
  });
}

// Convenience selectors that mirror the old flat store shape
export const useCells = () => useActiveTab().cells;
export const useSessionStatus = () => useActiveTab().sessionStatus;
export const useActiveCellId = () => useActiveTab().activeCellId;
export const useFilePath = () => useActiveTab().filePath;
export const useIsDirty = () => useActiveTab().isDirty;
export const useCanUndo = () => useActiveTab().canUndo;
export const useCanRedo = () => useActiveTab().canRedo;

/** Get active tab state from outside React (callbacks, event handlers). */
export function getActiveTabState(): NotebookTab {
  const state = useNotebookStore.getState();
  const id = state.activeNotebookId;
  if (!id) return createDefaultTab("");
  return state.notebooks[id] ?? createDefaultTab("");
}
