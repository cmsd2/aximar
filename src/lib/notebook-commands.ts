import { invoke } from "@tauri-apps/api/core";
import type { EvalResult } from "../types/maxima";

interface NbStateResult {
  effect: string;
  cell_id: string | null;
  cells: { id: string; cell_type: string; input: string; output?: unknown; status?: string }[];
  can_undo: boolean;
  can_redo: boolean;
}

export async function nbGetState(): Promise<NbStateResult> {
  return invoke<NbStateResult>("nb_get_state");
}

interface NbAddCellResult {
  cell_id: string;
}

export async function nbAddCell(
  cellType?: string,
  input?: string,
  afterCellId?: string
): Promise<NbAddCellResult> {
  return invoke<NbAddCellResult>("nb_add_cell", {
    cellType: cellType ?? null,
    input: input ?? null,
    afterCellId: afterCellId ?? null,
  });
}

export async function nbDeleteCell(cellId: string): Promise<void> {
  return invoke<void>("nb_delete_cell", { cellId });
}

export async function nbMoveCell(
  cellId: string,
  direction: "up" | "down"
): Promise<void> {
  return invoke<void>("nb_move_cell", { cellId, direction });
}

export async function nbToggleCellType(cellId: string): Promise<void> {
  return invoke<void>("nb_toggle_cell_type", { cellId });
}

export async function nbUpdateCellInput(
  cellId: string,
  input: string
): Promise<void> {
  return invoke<void>("nb_update_cell_input", { cellId, input });
}

export async function nbUndo(): Promise<void> {
  return invoke<void>("nb_undo");
}

export async function nbRedo(): Promise<void> {
  return invoke<void>("nb_redo");
}

export async function nbNewNotebook(): Promise<void> {
  return invoke<void>("nb_new_notebook");
}

export async function nbLoadCells(
  cells: { id: string; cell_type: string; input: string }[]
): Promise<void> {
  return invoke<void>("nb_load_cells", { cells });
}

export async function nbRunCell(cellId: string): Promise<EvalResult> {
  return invoke<EvalResult>("nb_run_cell", { cellId });
}
