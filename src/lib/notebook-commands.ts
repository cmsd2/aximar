import { invoke } from "@tauri-apps/api/core";
import type { EvalResult } from "../types/maxima";
import { useNotebookStore } from "../store/notebookStore";

/** Get the currently active notebook ID for passing to backend commands. */
function activeId(): string | null {
  return useNotebookStore.getState().activeNotebookId;
}

interface NbStateSyncCell {
  id: string;
  cell_type: string;
  input: string;
  output?: {
    text_output: string;
    latex: string | null;
    plot_svg: string | null;
    error: string | null;
    is_error: boolean;
    duration_ms: number;
    output_label: string | null;
    execution_count: number | null;
  } | null;
  status?: string | null;
}

interface NbStateResult {
  notebook_id: string;
  effect: string;
  cell_id: string | null;
  cells: NbStateSyncCell[];
  can_undo: boolean;
  can_redo: boolean;
}

export async function nbGetState(notebookId?: string): Promise<NbStateResult> {
  return invoke<NbStateResult>("nb_get_state", {
    notebookId: notebookId ?? activeId(),
  });
}

interface NbAddCellResult {
  cell_id: string;
}

export async function nbAddCell(
  cellType?: string,
  input?: string,
  afterCellId?: string,
  beforeCellId?: string
): Promise<NbAddCellResult> {
  return invoke<NbAddCellResult>("nb_add_cell", {
    notebookId: activeId(),
    cellType: cellType ?? null,
    input: input ?? null,
    afterCellId: afterCellId ?? null,
    beforeCellId: beforeCellId ?? null,
  });
}

export async function nbDeleteCell(cellId: string): Promise<void> {
  return invoke<void>("nb_delete_cell", { notebookId: activeId(), cellId });
}

export async function nbMoveCell(
  cellId: string,
  direction: "up" | "down"
): Promise<void> {
  return invoke<void>("nb_move_cell", { notebookId: activeId(), cellId, direction });
}

export async function nbToggleCellType(cellId: string): Promise<void> {
  return invoke<void>("nb_toggle_cell_type", { notebookId: activeId(), cellId });
}

export async function nbUpdateCellInput(
  cellId: string,
  input: string
): Promise<void> {
  return invoke<void>("nb_update_cell_input", { notebookId: activeId(), cellId, input });
}

export async function nbUndo(): Promise<void> {
  return invoke<void>("nb_undo", { notebookId: activeId() });
}

export async function nbRedo(): Promise<void> {
  return invoke<void>("nb_redo", { notebookId: activeId() });
}

export async function nbNewNotebook(): Promise<void> {
  return invoke<void>("nb_new_notebook", { notebookId: activeId() });
}

export async function nbLoadCells(
  cells: { id: string; cell_type: string; input: string }[],
  notebookId?: string,
): Promise<void> {
  return invoke<void>("nb_load_cells", { notebookId: notebookId ?? activeId(), cells });
}

export async function nbRunCell(cellId: string): Promise<EvalResult> {
  return invoke<EvalResult>("nb_run_cell", { notebookId: activeId(), cellId });
}

// ── Notebook lifecycle commands ──────────────────────────────────────

interface NbCreateResult {
  notebook_id: string;
}

export async function nbCreate(): Promise<NbCreateResult> {
  return invoke<NbCreateResult>("nb_create");
}

export async function nbClose(notebookId: string): Promise<void> {
  return invoke<void>("nb_close", { notebookId });
}

interface NotebookInfo {
  id: string;
  title: string;
  path: string | null;
  is_active: boolean;
}

export async function nbList(): Promise<NotebookInfo[]> {
  return invoke<NotebookInfo[]>("nb_list");
}

export async function nbSetActive(notebookId: string): Promise<void> {
  return invoke<void>("nb_set_active", { notebookId });
}
