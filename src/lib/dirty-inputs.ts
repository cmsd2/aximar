import { nbUpdateCellInput } from "./notebook-commands";

const DEBOUNCE_MS = 300;

interface PendingEdit {
  notebookId: string;
  cellId: string;
  input: string;
}

/** Edits not yet sent to the backend.  Each keeps the notebook it was made
 *  in, so switching tabs before the timer fires can't send it elsewhere. */
const dirtyInputs = new Map<string, PendingEdit>();
let timer: ReturnType<typeof setTimeout> | null = null;

function key(notebookId: string, cellId: string): string {
  return `${notebookId}\u0000${cellId}`;
}

function send(edit: PendingEdit): Promise<void> {
  return nbUpdateCellInput(edit.notebookId, edit.cellId, edit.input);
}

function scheduleFlush() {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => {
    timer = null;
    const edits = Array.from(dirtyInputs.values());
    dirtyInputs.clear();
    for (const edit of edits) {
      send(edit).catch((e) => console.warn("Input sync failed:", e));
    }
  }, DEBOUNCE_MS);
}

/** Record a dirty cell input and reset the debounce timer. */
export function markDirty(notebookId: string, cellId: string, input: string) {
  dirtyInputs.set(key(notebookId, cellId), { notebookId, cellId, input });
  scheduleFlush();
}

/** True when a cell has a local input edit that hasn't yet been flushed to
 *  the backend.  Used by the store's merge logic to distinguish a backend
 *  echo of our own debounced sync from an external mutation (e.g. MCP). */
export function hasPendingSync(notebookId: string, cellId: string): boolean {
  return dirtyInputs.has(key(notebookId, cellId));
}

/** Immediately sync one cell's dirty input to the backend. */
export async function flushCell(notebookId: string, cellId: string): Promise<void> {
  const k = key(notebookId, cellId);
  const edit = dirtyInputs.get(k);
  if (edit === undefined) return;
  dirtyInputs.delete(k);
  if (dirtyInputs.size === 0 && timer) {
    clearTimeout(timer);
    timer = null;
  }
  await send(edit);
}

/** Cancel the debounce timer (for hook teardown). */
export function cleanup() {
  if (timer) {
    clearTimeout(timer);
    timer = null;
  }
}
