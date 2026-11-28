// A key the cell editor binds never reaches the menu or the window: in a
// cell, the editor runs its own command instead.  So no menu shortcut may
// be one of the editor's keys.  (On macOS, Mod+Shift+Up moved cells until
// the editor turned out to take it, selecting to the start.)
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ ask: vi.fn(), message: vi.fn(), open: vi.fn(), save: vi.fn() }));

import { COMMANDS } from "./registry";
import { cellEditorKeymap } from "../hooks/useCodeMirrorEditor";

/** A key as a set of lower-case parts, so modifier order doesn't matter */
function parts(key: string, separator: string): string {
  return key
    .split(separator)
    .map((p) => p.toLowerCase())
    .sort()
    .join("+");
}

describe("menu shortcuts", () => {
  it("are none of the cell editor's own keys", () => {
    const editor = new Set(
      cellEditorKeymap().flatMap((b) => {
        const keys = b.key ? [parts(b.key, "-")] : [];
        // A binding's shift variant uses the same key with Shift
        if (b.key && b.shift) keys.push(parts(`Shift-${b.key}`, "-"));
        return keys;
      }),
    );
    const clashes = COMMANDS.filter((c) => c.shortcut && editor.has(parts(c.shortcut, "+"))).map(
      (c) => `${c.id}: ${c.shortcut}`,
    );
    expect(clashes).toEqual([]);
  });
});
