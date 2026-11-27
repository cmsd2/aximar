import type { Cell } from "../types/notebook";

/**
 * Compute the inclusive range of cell IDs between `anchor` and `target`
 * in the cell array. Returns an empty array if either ID is not found.
 */
export function computeRange(
  cells: Cell[],
  anchor: string,
  target: string,
): string[] {
  const anchorIdx = cells.findIndex((c) => c.id === anchor);
  const targetIdx = cells.findIndex((c) => c.id === target);
  if (anchorIdx === -1 || targetIdx === -1) return [];

  const lo = Math.min(anchorIdx, targetIdx);
  const hi = Math.max(anchorIdx, targetIdx);
  return cells.slice(lo, hi + 1).map((c) => c.id);
}

/** How a click changes the selection: plain, with Cmd or Ctrl, with Shift */
export type SelectHow = "only" | "toggle" | "range";

/** The click's modifier keys as a SelectHow */
export function selectHowOf(e: { shiftKey: boolean; metaKey: boolean; ctrlKey: boolean }): SelectHow {
  if (e.shiftKey) return "range";
  if (e.metaKey || e.ctrlKey) return "toggle";
  return "only";
}

/**
 * The selection after clicking `cellId`. A plain click selects that cell
 * alone, or clears the selection if it was the only one selected. Cmd or
 * Ctrl adds or removes one cell. Shift selects from the last cell
 * selected to this one.
 */
export function nextSelection(cells: Cell[], selected: string[], cellId: string, how: SelectHow): string[] {
  if (how === "range" && selected.length > 0) {
    const range = computeRange(cells, selected[selected.length - 1], cellId);
    if (range.length > 0) return range;
  }
  if (how === "toggle") {
    return selected.includes(cellId) ? selected.filter((id) => id !== cellId) : [...selected, cellId];
  }
  return selected.length === 1 && selected[0] === cellId ? [] : [cellId];
}
