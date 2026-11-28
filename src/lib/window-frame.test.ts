import { describe, expect, it } from "vitest";
import { drawsMenuBar, drawsTitleBar } from "./window-frame";

describe("who draws the window's frame", () => {
  it("draws the title bar, with the tabs, on Windows and macOS", () => {
    expect(drawsTitleBar("windows")).toBe(true);
    expect(drawsTitleBar("macos")).toBe(true);
    expect(drawsTitleBar("linux")).toBe(false);
  });

  it("draws the menu bar only on Windows; macOS and Linux have the system's", () => {
    expect(drawsMenuBar("windows")).toBe(true);
    expect(drawsMenuBar("macos")).toBe(false);
    expect(drawsMenuBar("linux")).toBe(false);
  });
});
