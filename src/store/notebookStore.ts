import { create } from "zustand";
import type { Cell } from "../types/notebook";
import type { SessionStatus } from "../types/maxima";
import { computeRange } from "../lib/selection-utils";

export type Theme = "auto" | "light" | "dark";
export type CellStyle = "card" | "bracket";
export type AutocompleteMode = "hint" | "snippet" | "active-hint";

export interface NotebookTab {
  type: "notebook";
  id: string;
  title: string;
  cells: Cell[];
  sessionStatus: SessionStatus;
  activeCellId: string | null;
  selectedCellIds: string[];
  executionCounter: number;
  filePath: string | null;
  isDirty: boolean;
  canUndo: boolean;
  canRedo: boolean;
  closePending: boolean;
  pendingCursorMove: { cellId: string; pos: number } | null;
  /** Whether the notebook is trusted for dangerous function execution. */
  trusted: boolean;
  /** Dangerous functions detected when the notebook needs trust. Drives the trust banner. */
  pendingTrustFunctions: string[] | null;
}

export interface PlotTab {
  type: "plot";
  id: string;
  title: string;
  filePath: string | null;
  plotData: string;
  isDirty: false;
  closePending: boolean;
}

export type Tab = NotebookTab | PlotTab;

function createDefaultTab(id: string): NotebookTab {
  return {
    type: "notebook",
    id,
    title: "Untitled",
    cells: [],
    sessionStatus: "Stopped",
    activeCellId: null,
    selectedCellIds: [],
    executionCounter: 0,
    filePath: null,
    isDirty: false,
    canUndo: false,
    canRedo: false,
    closePending: false,
    pendingCursorMove: null,
    trusted: true,
    pendingTrustFunctions: null,
  };
}

interface NotebookState {
  notebooks: Record<string, Tab>;
  activeNotebookId: string | null;

  // Global settings (not per-notebook)
  theme: Theme;
  cellStyle: CellStyle;
  autocompleteMode: AutocompleteMode;

  // --- Tab lifecycle ---
  addTab: (id: string, title?: string) => void;
  addPlotTab: (id: string, title: string, plotData: string, filePath: string | null) => void;
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
  setSelectedCellIds: (ids: string[]) => void;
  clearSelection: () => void;
  toggleCellSelected: (cellId: string, range?: boolean) => void;
  setFilePath: (path: string | null) => void;
  setClosePending: (notebookId: string, pending: boolean) => void;
  setPendingTrustFunctions: (functions: string[] | null) => void;
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
    canRedo?: boolean,
    trusted?: boolean
  ) => void;
}

/** Update a notebook tab within the state. No-ops for non-notebook tabs. */
function updateNotebookTab(
  notebooks: Record<string, Tab>,
  id: string,
  updater: (tab: NotebookTab) => Partial<NotebookTab>
): Record<string, Tab> {
  const tab = notebooks[id];
  if (!tab || tab.type !== "notebook") return notebooks;
  return { ...notebooks, [id]: { ...tab, ...updater(tab) } };
}

/** Update any tab (shared fields like closePending). */
function updateAnyTab(
  notebooks: Record<string, Tab>,
  id: string,
  updater: (tab: Tab) => Partial<Tab>
): Record<string, Tab> {
  const tab = notebooks[id];
  if (!tab) return notebooks;
  return { ...notebooks, [id]: { ...tab, ...updater(tab) } as Tab };
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

  addPlotTab: (id: string, title: string, plotData: string, filePath: string | null) =>
    set((state) => {
      const tab: PlotTab = {
        type: "plot",
        id,
        title,
        plotData,
        filePath,
        isDirty: false,
        closePending: false,
      };
      return {
        notebooks: { ...state.notebooks, [id]: tab },
        activeNotebookId: id,
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
        notebooks: updateNotebookTab(state.notebooks, nbId, (tab) => ({
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
      if (!tab || tab.type !== "notebook" || !tab.activeCellId) return state;
      return {
        notebooks: updateNotebookTab(state.notebooks, nbId, (tab) => ({
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
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
          sessionStatus: status,
        })),
      };
    }),

  setSessionStatusForNotebook: (notebookId: string, status: SessionStatus) =>
    set((state) => ({
      notebooks: updateNotebookTab(state.notebooks, notebookId, () => ({
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
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
          activeCellId: id,
        })),
      };
    }),

  setSelectedCellIds: (ids: string[]) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
          selectedCellIds: ids,
        })),
      };
    }),

  clearSelection: () =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
          selectedCellIds: [],
        })),
      };
    }),

  toggleCellSelected: (cellId: string, range?: boolean) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      const tab = state.notebooks[nbId];
      if (!tab || tab.type !== "notebook") return state;

      let newSelected: string[];
      if (range && tab.selectedCellIds.length > 0) {
        // Shift-click: select range from last selected to target
        const anchor = tab.selectedCellIds[tab.selectedCellIds.length - 1];
        newSelected = computeRange(tab.cells, anchor, cellId);
      } else {
        // Toggle single cell
        if (tab.selectedCellIds.includes(cellId)) {
          newSelected = tab.selectedCellIds.filter((id) => id !== cellId);
        } else {
          newSelected = [...tab.selectedCellIds, cellId];
        }
      }

      return {
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
          selectedCellIds: newSelected,
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
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
          filePath: path,
          title,
        })),
      };
    }),

  setClosePending: (notebookId: string, pending: boolean) =>
    set((state) => ({
      notebooks: updateAnyTab(state.notebooks, notebookId, () => ({
        closePending: pending,
      })),
    })),

  setPendingTrustFunctions: (functions: string[] | null) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
          pendingTrustFunctions: functions,
        })),
      };
    }),

  markClean: () =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
          isDirty: false,
        })),
      };
    }),

  setPendingCursorMove: (move: { cellId: string; pos: number }) =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
          pendingCursorMove: move,
        })),
      };
    }),

  clearPendingCursorMove: () =>
    set((state) => {
      const nbId = state.activeNotebookId;
      if (!nbId) return state;
      return {
        notebooks: updateNotebookTab(state.notebooks, nbId, () => ({
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
    canRedo?: boolean,
    trusted?: boolean
  ) =>
    set((state) => {
      const tab = state.notebooks[notebookId];
      if (!tab || tab.type !== "notebook") return state;

      // Merge backend cells with local state, preserving local input edits.
      // For cell_input_updated events (echoes of frontend debounced syncs),
      // always keep the local input — it may have advanced past what the
      // backend echoed back.  Undo/redo use distinct effects (undone/redone)
      // and fall through to useBackendInput via isReplace/!localCell.
      const isReplace = effect === "notebook_replaced";
      const mergedCells = cells.map((backendCell) => {
        const localCell = tab.cells.find((c) => c.id === backendCell.id);
        const useBackendInput = isReplace || !localCell;
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
        trusted: trusted ?? tab.trusted,
        // Clear pending trust prompt when notebook becomes trusted
        pendingTrustFunctions: (trusted ?? tab.trusted) ? null : tab.pendingTrustFunctions,
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

/** Return the active tab (any type), or null. */
export function useActiveAnyTab(): Tab | null {
  return useNotebookStore((s) => {
    const id = s.activeNotebookId;
    if (!id) return null;
    return s.notebooks[id] ?? null;
  });
}

/** Return the active notebook tab, or a stable empty tab. Non-notebook tabs return EMPTY_TAB. */
export function useActiveTab(): NotebookTab {
  return useNotebookStore((s) => {
    const id = s.activeNotebookId;
    if (!id) return EMPTY_TAB;
    const tab = s.notebooks[id];
    if (!tab || tab.type !== "notebook") return EMPTY_TAB;
    return tab;
  });
}

// Convenience selectors that mirror the old flat store shape
export const useCells = () => useActiveTab().cells;
export const useSessionStatus = () => useActiveTab().sessionStatus;
export const useActiveCellId = () => useActiveTab().activeCellId;
export const useSelectedCellIds = () => useActiveTab().selectedCellIds;
export const useFilePath = () => useActiveTab().filePath;
export const useIsDirty = () => useActiveTab().isDirty;
export const useCanUndo = () => useActiveTab().canUndo;
export const useCanRedo = () => useActiveTab().canRedo;
export const useTrusted = () => useActiveTab().trusted;

/** Get active tab state from outside React (callbacks, event handlers). */
export function getActiveTabState(): NotebookTab {
  const state = useNotebookStore.getState();
  const id = state.activeNotebookId;
  if (!id) return createDefaultTab("");
  const tab = state.notebooks[id];
  if (!tab || tab.type !== "notebook") return createDefaultTab("");
  return tab;
}
