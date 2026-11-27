// The caret between cells: where the arrow keys go from a cell's edge or
// from a gap, and starting a new cell at a gap.  See CellGap.
import { getActiveTabState, useNotebookStore, type InsertionPoint } from "../store/notebookStore";
import { nbAddCell } from "./notebook-commands";
import type { Cell } from "../types/notebook";

/** Where an arrow key goes: into a cell, at its start or end, or to a gap */
export type CaretTarget =
  | { kind: "cell"; cellId: string; at: "start" | "end" }
  | { kind: "gap"; point: InsertionPoint };

/** The gap above or below a cell, for an arrow key at the cell's first or last line */
export function gapBeside(cells: Cell[], cellId: string, dir: "up" | "down"): InsertionPoint | null {
  const i = cells.findIndex((c) => c.id === cellId);
  if (i === -1) return null;
  if (dir === "down") return { after: cellId };
  return { after: i === 0 ? null : cells[i - 1].id };
}

/**
 * Where an arrow key goes from a gap: up into the cell above it, at its
 * end, or down into the cell below, at its start.  A raw cell can't take
 * the cursor, so the caret moves on to the gap beyond it.  Null when there
 * is nothing that way.
 */
export function fromGap(cells: Cell[], point: InsertionPoint, dir: "up" | "down"): CaretTarget | null {
  const above = point.after === null ? -1 : cells.findIndex((c) => c.id === point.after);
  if (point.after !== null && above === -1) return null;
  const i = dir === "up" ? above : above + 1;
  const cell = cells[i];
  if (!cell) return null;
  if (cell.cellType === "raw") {
    return { kind: "gap", point: dir === "down" ? { after: cell.id } : { after: i === 0 ? null : cells[i - 1].id } };
  }
  return { kind: "cell", cellId: cell.id, at: dir === "up" ? "end" : "start" };
}

/**
 * Where Shift+Enter goes after running a cell, or after finishing a
 * Markdown cell: into the next cell at its start, or, after the last
 * cell, to the caret in the gap below it, ready to start a new one.
 */
export function afterRun(cells: Cell[], cellId: string): CaretTarget {
  return fromGap(cells, { after: cellId }, "down") ?? { kind: "gap", point: { after: cellId } };
}

/** Whether new text at a gap starts a Markdown cell: a heading, as Maxima code never starts with # */
export function typeFor(text: string): "code" | "markdown" {
  return text.startsWith("#") ? "markdown" : "code";
}

/** Put the cursor in a cell, at `pos` (clamped to its text) */
export function focusCell(cellId: string, pos: number): void {
  useNotebookStore.getState().setActiveCellId(cellId);
  useNotebookStore.getState().setPendingCursorMove({ cellId, pos });
}

/** Go to a target: a cell's start or end, or a gap */
export function goTo(target: CaretTarget): void {
  const store = useNotebookStore.getState();
  if (target.kind === "gap") {
    store.setInsertionPoint(target.point);
    return;
  }
  store.setInsertionPoint(null);
  const cell = getActiveTabState().cells.find((c) => c.id === target.cellId);
  focusCell(target.cellId, target.at === "start" ? 0 : (cell?.input.length ?? 0));
}

/**
 * Add a cell at a gap with `input`, and put the cursor at its end.  `more`
 * gives the text typed while the backend added the cell, which the new
 * cell takes so no keystroke is lost.
 */
export async function insertAt(
  point: InsertionPoint,
  cellType: "code" | "markdown",
  input = "",
  more?: () => string,
): Promise<string> {
  const cells = getActiveTabState().cells;
  const before = point.after === null ? cells[0]?.id : undefined;
  const { cell_id } = await nbAddCell(cellType, input, point.after ?? undefined, before);
  const store = useNotebookStore.getState();
  let text = input;
  const later = more?.();
  if (later && later !== input) {
    text = later;
    store.updateCellInput(cell_id, text);
  }
  store.setInsertionPoint(null);
  focusCell(cell_id, text.length);
  return cell_id;
}
