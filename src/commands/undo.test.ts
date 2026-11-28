import { afterEach, describe, expect, it, vi } from "vitest";

const calls = vi.hoisted(() => [] as string[]);

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, args: Record<string, unknown>) => {
    calls.push(cmd === "nb_update_cell_input" ? `${cmd}:${String(args.input)}` : cmd);
  }),
}));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

import { undoOrRedo } from "./registry";
import { cleanup, hasPendingSync, markDirty } from "../lib/dirty-inputs";
import { useNotebookStore } from "../store/notebookStore";

afterEach(() => {
  cleanup();
  calls.length = 0;
});

describe("undo and redo", () => {
  it.each(["undo", "redo"] as const)("sends waiting typing before they %s", async (which) => {
    useNotebookStore.setState({ activeNotebookId: "nb-a" });
    // A line copied with Alt+Shift+Up a moment ago, not yet sent
    markDirty("nb-a", "cell-1", "b: 2$\nb: 2$");

    undoOrRedo(which);
    await vi.waitFor(() => expect(calls).toContain(`nb_${which}`));

    expect(calls).toEqual([`nb_update_cell_input:b: 2$\nb: 2$`, `nb_${which}`]);
    expect(hasPendingSync("nb-a", "cell-1")).toBe(false);
  });

  it("undoes once when one key press reaches both the editor and the menu", async () => {
    vi.useFakeTimers();
    try {
      useNotebookStore.setState({ activeNotebookId: "nb-a" });
      undoOrRedo("undo", "editor");
      undoOrRedo("undo", "menu");
      await vi.runAllTimersAsync();
      expect(calls.filter((c) => c === "nb_undo")).toHaveLength(1);

      // Ctrl+Z held down: each repeat from the editor undoes
      calls.length = 0;
      vi.advanceTimersByTime(1000);
      undoOrRedo("undo", "editor");
      undoOrRedo("undo", "editor");
      await vi.runAllTimersAsync();
      expect(calls.filter((c) => c === "nb_undo")).toHaveLength(2);
    } finally {
      vi.useRealTimers();
    }
  });
});
