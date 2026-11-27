import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

import { COMMANDS } from "./registry";
import { toAccelerator } from "./shortcut";

describe("the command list", () => {
  it("has unique ids", () => {
    const ids = COMMANDS.map((c) => c.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("has unique shortcuts, menu and editor ones together", () => {
    const keys = COMMANDS.flatMap((c) => [c.shortcut, c.editorShortcut]).filter(Boolean);
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("gives every label a translator note", () => {
    for (const c of COMMANDS) {
      for (const label of [c.label, c.contextLabel].filter(Boolean)) {
        expect(label!.comment, c.id).toBeTruthy();
      }
    }
  });

  it("binds no accelerator for Run and Run and Stay, whose keys the editor handles", () => {
    const run = COMMANDS.find((c) => c.id === "cell.run")!;
    const stay = COMMANDS.find((c) => c.id === "cell.run_stay")!;
    expect(run.shortcut).toBeUndefined();
    expect(stay.shortcut).toBeUndefined();
    expect(run.editorShortcut).toBe("Shift+Enter");
    expect(stay.editorShortcut).toBe("Mod+Enter");
  });

  it("gives every shortcut an accelerator", () => {
    for (const c of COMMANDS) if (c.shortcut) expect(() => toAccelerator(c.shortcut!), c.id).not.toThrow();
  });

  it("no longer uses Mod+D or Mod+T", () => {
    const keys = COMMANDS.map((c) => c.shortcut);
    expect(keys).not.toContain("Mod+D");
    expect(keys).not.toContain("Mod+T");
  });
});
