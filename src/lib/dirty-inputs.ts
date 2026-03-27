import { nbUpdateCellInput } from "./notebook-commands";

const DEBOUNCE_MS = 300;

const dirtyInputs = new Map<string, string>();
let timer: ReturnType<typeof setTimeout> | null = null;

function scheduleFlush() {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => {
    timer = null;
    const entries = Array.from(dirtyInputs);
    dirtyInputs.clear();
    for (const [cellId, input] of entries) {
      nbUpdateCellInput(cellId, input).catch((e) =>
        console.warn("Input sync failed:", e)
      );
    }
  }, DEBOUNCE_MS);
}

/** Record a dirty cell input and reset the debounce timer. */
export function markDirty(cellId: string, input: string) {
  dirtyInputs.set(cellId, input);
  scheduleFlush();
}

/** Immediately sync one cell's dirty input to the backend. */
export async function flushCell(cellId: string): Promise<void> {
  const input = dirtyInputs.get(cellId);
  if (input === undefined) return;
  dirtyInputs.delete(cellId);
  if (dirtyInputs.size === 0 && timer) {
    clearTimeout(timer);
    timer = null;
  }
  await nbUpdateCellInput(cellId, input);
}

/** Immediately sync all dirty inputs to the backend. */
export async function flushAll(): Promise<void> {
  if (timer) {
    clearTimeout(timer);
    timer = null;
  }
  const entries = Array.from(dirtyInputs);
  dirtyInputs.clear();
  await Promise.all(
    entries.map(([cellId, input]) =>
      nbUpdateCellInput(cellId, input).catch((e) =>
        console.warn("Input sync failed:", e)
      )
    )
  );
}

/** Cancel the debounce timer (for hook teardown). */
export function cleanup() {
  if (timer) {
    clearTimeout(timer);
    timer = null;
  }
}
