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
