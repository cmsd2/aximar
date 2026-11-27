import { invoke } from "@tauri-apps/api/core";
import type { ErrorInfo, EvalResult, Note } from "../types/maxima";
import { useNotebookStore } from "../store/notebookStore";

/** Get the currently active notebook ID for passing to backend commands. */
function activeId(): string | null {
  return useNotebookStore.getState().activeNotebookId;
}

/** A cell's output as the backend sends it (aximar-core's CellOutput). */
export interface SyncCellOutput {
  text_output: string;
  latex: string | null;
  plot_svg: string | null;
  plot_data: string | null;
  image_png: string | null;
  error: string | null;
  is_error: boolean;
  duration_ms: number;
  output_label: string | null;
  execution_count: number | null;
  /** Maxima's one-line form of the result */
  text_plain: string | null;
  error_info: ErrorInfo | null;
  /** Maxima's error message in the user's language, when it has one */
  error_translated?: string | null;
  /** What Maxima's notes in the text output mean */
  notes?: Note[];
  /** Read from a file, from an earlier session */
  stale: boolean;
}

/** A cell as the backend sends it in state events and nb_get_state. */
export interface SyncCell {
  id: string;
  cell_type: string;
  input: string;
  output?: SyncCellOutput | null;
  status?: string | null;
  /** The input's last statement ends in `$` */
  silent?: boolean;
}

interface NbStateResult {
  notebook_id: string;
  effect: string;
  cell_id: string | null;
  cells: SyncCell[];
  can_undo: boolean;
  can_redo: boolean;
  trusted: boolean;
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

/** Takes the notebook explicitly: edits are sent after a delay, by which
 *  time the active notebook may have changed. */
export async function nbUpdateCellInput(
  notebookId: string,
  cellId: string,
  input: string
): Promise<void> {
  return invoke<void>("nb_update_cell_input", { notebookId, cellId, input });
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

/** Open a notebook file into a notebook.  The backend reads the file, loads
 *  the cells and records the path; events then update the tab. */
export async function nbOpen(path: string, notebookId?: string): Promise<void> {
  return invoke<void>("nb_open", { notebookId: notebookId ?? activeId(), path });
}

/** Save a notebook to a file and record the path. */
export async function nbSave(path: string, notebookId?: string): Promise<void> {
  return invoke<void>("nb_save", { notebookId: notebookId ?? activeId(), path });
}

/** Load a built-in template into a notebook. */
export async function nbLoadTemplate(templateId: string, notebookId?: string): Promise<void> {
  return invoke<void>("nb_load_template", { notebookId: notebookId ?? activeId(), templateId });
}

export type RunCellResult =
  | { type: "evaluated" } & EvalResult
  | { type: "needs_notebook_trust"; dangerous_functions: string[] };

export async function nbRunCell(cellId: string): Promise<RunCellResult> {
  return invoke<RunCellResult>("nb_run_cell", { notebookId: activeId(), cellId });
}

export type CellRunStatus = "ok" | "empty" | "error" | "needs_trust" | "failed" | "not_run";

/** One code cell's outcome in a run of all cells. */
export interface CellRun {
  cell_id: string;
  status: CellRunStatus;
  execution_count?: number;
  error?: string;
  dangerous_functions?: string[];
  duration_ms?: number;
}

/** What `nb_run_all` reports: one entry per code cell, in order. */
export interface RunAllReport {
  cells: CellRun[];
  stopped_at: string | null;
  stop_reason: "error" | "needs_trust" | "failed" | "cancelled" | null;
}

/** Run every code cell, stopping at the first that doesn't succeed. */
export async function nbRunAll(notebookId: string): Promise<RunAllReport> {
  return invoke<RunAllReport>("nb_run_all", { notebookId });
}

export async function nbTrustNotebook(trusted: boolean): Promise<void> {
  return invoke<void>("nb_trust_notebook", { notebookId: activeId(), trusted });
}

// ── Notebook lifecycle commands ──────────────────────────────────────

interface NbCreateResult {
  notebook_id: string;
}

/** Create a notebook: with one empty code cell, or with `empty` none, as New gives */
export async function nbCreate(options: { empty?: boolean } = {}): Promise<NbCreateResult> {
  return invoke<NbCreateResult>("nb_create", { empty: options.empty ?? null });
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

/** Record the file a notebook was opened from or saved to, or clear it.
 *  The backend's path_changed event updates the tab's title and save path. */
export async function nbSetPath(notebookId: string, path: string | null): Promise<void> {
  return invoke<void>("nb_set_path", { notebookId, path });
}

export async function nbList(): Promise<NotebookInfo[]> {
  return invoke<NotebookInfo[]>("nb_list");
}

export async function nbSetActive(notebookId: string): Promise<void> {
  return invoke<void>("nb_set_active", { notebookId });
}

// ── Startup commands ─────────────────────────────────────────────────

/** Get CLI file arguments from initial launch (take-once). */
export async function getInitialFileArgs(): Promise<string[] | null> {
  return invoke<string[] | null>("get_initial_file_args");
}

/** Take the files the system asked Aximar to open (a drop on the Dock icon, Open With, a double-click) */
export async function takePendingOpens(): Promise<string[]> {
  return invoke<string[]>("take_pending_opens");
}

// ── Window commands ──────────────────────────────────────────────────

export async function createWindow(notebookId?: string): Promise<string> {
  return invoke<string>("create_window", { notebookId: notebookId ?? null });
}
