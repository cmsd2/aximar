// @vitest-environment happy-dom
import { afterEach, describe, expect, it } from "vitest";
import { DEFAULT_LAYOUT, drawsMenuBar, drawsTitleBar, parseDecorationLayout } from "./window-frame";

describe("who draws the window's frame", () => {
  afterEach(() => {
    delete window.__AXIMAR_PAGE_FRAME__;
  });

  it("draws the title bar, with the tabs, on Windows and macOS", () => {
    expect(drawsTitleBar("windows")).toBe(true);
    expect(drawsTitleBar("macos")).toBe(true);
  });

  it("draws the menu bar on Windows; macOS has the system's", () => {
    expect(drawsMenuBar("windows")).toBe(true);
    expect(drawsMenuBar("macos")).toBe(false);
  });

  it("draws both on Linux when the backend says so, and neither otherwise", () => {
    expect(drawsTitleBar("linux")).toBe(false);
    expect(drawsMenuBar("linux")).toBe(false);
    window.__AXIMAR_PAGE_FRAME__ = true;
    expect(drawsTitleBar("linux")).toBe(true);
    expect(drawsMenuBar("linux")).toBe(true);
  });

  it("leaves macOS's menu bar to the system whatever the backend says", () => {
    window.__AXIMAR_PAGE_FRAME__ = true;
    expect(drawsMenuBar("macos")).toBe(false);
  });
});

describe("the title bar's layout", () => {
  it("follows KDE's: the icon first, the buttons last", () => {
    expect(parseDecorationLayout("icon:minimize,maximize,close")).toEqual({
      start: ["icon"],
      end: ["minimize", "maximize", "close"],
    });
  });

  it("follows GNOME's: close alone, without the app menu", () => {
    expect(parseDecorationLayout("appmenu:close")).toEqual({ start: [], end: ["close"] });
    expect(parseDecorationLayout(":close")).toEqual({ start: [], end: ["close"] });
  });

  it("puts buttons at the start where the desktop does", () => {
    expect(parseDecorationLayout("close,minimize,maximize:")).toEqual({
      start: ["close", "minimize", "maximize"],
      end: [],
    });
  });

  it("skips spaces and items it doesn't draw", () => {
    expect(parseDecorationLayout(" menu , icon : spacer, close ")).toEqual({ start: ["icon"], end: ["close"] });
  });

  it("uses Windows' layout until the desktop gives one", () => {
    expect(parseDecorationLayout(null)).toBe(DEFAULT_LAYOUT);
    expect(parseDecorationLayout(undefined)).toBe(DEFAULT_LAYOUT);
    expect(parseDecorationLayout("close")).toBe(DEFAULT_LAYOUT);
  });
});
