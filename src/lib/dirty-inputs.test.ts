import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const sent = vi.hoisted(() => [] as [string, string, string][]);

vi.mock("./notebook-commands", () => ({
  nbUpdateCellInput: vi.fn(async (notebookId: string, cellId: string, input: string) => {
    sent.push([notebookId, cellId, input]);
  }),
}));

import { cleanup, flushCell, flushNotebook, fromBackend, hasPendingSync, markDirty } from "./dirty-inputs";

beforeEach(() => {
  vi.useFakeTimers();
  sent.length = 0;
});

afterEach(() => {
  // Send anything left so the next test starts with no pending edits
  vi.runAllTimers();
  cleanup();
  vi.useRealTimers();
});

describe("dirty inputs", () => {
  it("sends an edit to the notebook it was made in", () => {
    markDirty("nb-a", "cell-1", "x^2");
    // The user switches tabs before the debounce fires; nothing here reads
    // the active notebook, so the edit still goes to nb-a
    vi.advanceTimersByTime(300);
    expect(sent).toEqual([["nb-a", "cell-1", "x^2"]]);
  });

  it("keeps edits to the same cell id in different notebooks apart", () => {
    markDirty("nb-a", "cell-1", "a");
    markDirty("nb-b", "cell-1", "b");
    vi.advanceTimersByTime(300);
    expect(sent).toHaveLength(2);
    expect(sent).toContainEqual(["nb-a", "cell-1", "a"]);
    expect(sent).toContainEqual(["nb-b", "cell-1", "b"]);
  });

  it("sends only the latest input for a cell", () => {
    markDirty("nb-a", "cell-1", "x");
    markDirty("nb-a", "cell-1", "x+1");
    vi.advanceTimersByTime(300);
    expect(sent).toEqual([["nb-a", "cell-1", "x+1"]]);
  });

  it("reports pending edits per notebook", () => {
    markDirty("nb-a", "cell-1", "x");
    expect(hasPendingSync("nb-a", "cell-1")).toBe(true);
    expect(hasPendingSync("nb-b", "cell-1")).toBe(false);
    vi.advanceTimersByTime(300);
    expect(hasPendingSync("nb-a", "cell-1")).toBe(false);
  });

  it("flushes one cell now and leaves the others for the timer", async () => {
    markDirty("nb-a", "cell-1", "one");
    markDirty("nb-a", "cell-2", "two");
    await flushCell("nb-a", "cell-1");
    expect(sent).toEqual([["nb-a", "cell-1", "one"]]);
    vi.advanceTimersByTime(300);
    expect(sent).toEqual([
      ["nb-a", "cell-1", "one"],
      ["nb-a", "cell-2", "two"],
    ]);
  });

  it("flushes one notebook's edits now and leaves the others on the timer", async () => {
    markDirty("nb-a", "cell-1", "a1");
    markDirty("nb-a", "cell-2", "a2");
    markDirty("nb-b", "cell-1", "b1");
    await flushNotebook("nb-a");
    expect(sent).toHaveLength(2);
    expect(sent).toContainEqual(["nb-a", "cell-1", "a1"]);
    expect(sent).toContainEqual(["nb-a", "cell-2", "a2"]);
    expect(hasPendingSync("nb-a", "cell-1")).toBe(false);
    expect(hasPendingSync("nb-b", "cell-1")).toBe(true);

    vi.advanceTimersByTime(300);
    expect(sent).toContainEqual(["nb-b", "cell-1", "b1"]);
    expect(sent).toHaveLength(3);
  });

  it("doesn't count text from the backend, such as an undo's, as an edit", () => {
    const result = fromBackend(() => {
      markDirty("nb-a", "cell-1", "undone");
      return 7;
    });
    expect(result).toBe(7);
    // Pending, it would keep a second undo's text out of the editor
    expect(hasPendingSync("nb-a", "cell-1")).toBe(false);
    vi.advanceTimersByTime(300);
    expect(sent).toEqual([]);

    // Edits after it count again
    markDirty("nb-a", "cell-1", "typed");
    expect(hasPendingSync("nb-a", "cell-1")).toBe(true);
  });
});
