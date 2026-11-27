import { describe, expect, it } from "vitest";
import { formatShortcut, matches, toAccelerator } from "./shortcut";

const key = (k: string, mods: Partial<Record<"metaKey" | "ctrlKey" | "altKey" | "shiftKey", boolean>> = {}) => ({
  key: k, metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...mods,
});

describe("shortcuts", () => {
  it("formats for macOS, symbols in Apple's order", () => {
    expect(formatShortcut("Mod+Shift+W", "macos")).toBe("⇧⌘W");
    expect(formatShortcut("Ctrl+Alt+Shift+Mod+K", "macos")).toBe("⌃⌥⇧⌘K");
    expect(formatShortcut("Mod+Shift+Backspace", "macos")).toBe("⇧⌘⌫");
    expect(formatShortcut("Shift+Enter", "macos")).toBe("⇧↩");
    expect(formatShortcut("Mod+.", "macos")).toBe("⌘.");
  });

  it("formats for Windows and Linux, with no Cmd", () => {
    expect(formatShortcut("Mod+Shift+W", "windows")).toBe("Ctrl+Shift+W");
    expect(formatShortcut("Mod+.", "windows")).toBe("Ctrl+.");
    expect(formatShortcut("Mod+Shift+ArrowUp", "linux")).toBe("Ctrl+Shift+Up");
    expect(formatShortcut("Shift+Enter", "linux")).toBe("Shift+Enter");
  });

  it("gives Tauri's accelerators", () => {
    expect(toAccelerator("Mod+Shift+W")).toBe("CmdOrCtrl+Shift+W");
    expect(toAccelerator("Mod+.")).toBe("CmdOrCtrl+Period");
    expect(toAccelerator("Mod+Shift+ArrowDown")).toBe("CmdOrCtrl+Shift+Down");
    expect(toAccelerator("Mod+Shift+]")).toBe("CmdOrCtrl+Shift+BracketRight");
    expect(toAccelerator("Mod+/")).toBe("CmdOrCtrl+Slash");
  });

  it("matches a key press, per platform", () => {
    expect(matches("Mod+W", key("w", { metaKey: true }), "macos")).toBe(true);
    expect(matches("Mod+W", key("w", { ctrlKey: true }), "macos")).toBe(false);
    expect(matches("Mod+W", key("w", { ctrlKey: true }), "windows")).toBe(true);
    expect(matches("Ctrl+Tab", key("Tab", { ctrlKey: true }), "macos")).toBe(true);
    expect(matches("Ctrl+Shift+Tab", key("Tab", { ctrlKey: true }), "macos")).toBe(false);
  });

  it("refuses an unknown modifier", () => {
    expect(() => formatShortcut("Cmd+W", "macos")).toThrow(/Unknown modifier/);
  });
});
