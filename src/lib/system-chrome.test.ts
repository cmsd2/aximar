import { describe, expect, it } from "vitest";
import { systemChromeVariables, type SystemChrome } from "./system-chrome";

const breeze: SystemChrome = {
  background: "#dee0e2",
  backdropBackground: "#eff0f1",
  border: "#bcbebf",
  dark: false,
  fontFamily: "Noto Sans",
  titleBackground: "#e3e5e7",
  titleBackdropBackground: "#eff0f1",
  decorationLayout: "icon:minimize,maximize,close",
  desktop: "kde",
};

const breezeDark: SystemChrome = {
  ...breeze,
  background: "#292c30",
  backdropBackground: "#202326",
  titleBackground: "#2a2e32",
  titleBackdropBackground: "#202326",
  dark: true,
};

const light = { theme: "light", systemDark: false, focused: true } as const;

describe("the desktop's chrome", () => {
  it("paints the tab bar and toolbar in the menu bar's colours, the title bar in the title bar's", () => {
    expect(systemChromeVariables(breeze, light)).toEqual({
      "--chrome-bg": "#dee0e2",
      "--chrome-border": "#bcbebf",
      "--chrome-title-bg": "#e3e5e7",
      "--font-ui": '"Noto Sans", sans-serif',
    });
  });

  it("dims with the menu bar and title bar when another window is active", () => {
    const vars = systemChromeVariables(breeze, { ...light, focused: false });
    expect(vars["--chrome-bg"]).toBe("#eff0f1");
    expect(vars["--chrome-title-bg"]).toBe("#eff0f1");
  });

  it("keeps the page's colours when the desktop is the other way round", () => {
    const vars = systemChromeVariables(breeze, { ...light, theme: "dark" });
    expect(vars["--chrome-bg"]).toBeUndefined();
    expect(vars["--chrome-border"]).toBeUndefined();
    expect(vars["--chrome-title-bg"]).toBeUndefined();
    expect(vars["--font-ui"]).toBe('"Noto Sans", sans-serif');
    expect(systemChromeVariables(breezeDark, light)["--chrome-bg"]).toBeUndefined();
  });

  it("follows the system's light or dark in Auto", () => {
    expect(systemChromeVariables(breezeDark, { ...light, theme: "auto", systemDark: true })["--chrome-bg"]).toBe(
      "#292c30",
    );
    expect(systemChromeVariables(breeze, { ...light, theme: "auto", systemDark: true })["--chrome-bg"]).toBeUndefined();
    expect(systemChromeVariables(breeze, { ...light, theme: "auto" })["--chrome-bg"]).toBe("#dee0e2");
  });

  it("leaves Paper's and Night's tinted greys alone", () => {
    expect(systemChromeVariables(breeze, { ...light, theme: "paper" })["--chrome-bg"]).toBeUndefined();
    expect(systemChromeVariables(breezeDark, { ...light, theme: "night" })["--chrome-bg"]).toBeUndefined();
  });

  it("sets nothing without a desktop theme, as on macOS and Windows", () => {
    expect(systemChromeVariables(null, light)).toEqual({});
  });

  it("skips a border and font the theme doesn't give", () => {
    expect(systemChromeVariables({ ...breeze, border: null, fontFamily: null }, light)).toEqual({
      "--chrome-bg": "#dee0e2",
      "--chrome-title-bg": "#e3e5e7",
    });
  });

  it("quotes a font family safely", () => {
    expect(systemChromeVariables({ ...breeze, fontFamily: 'A "B" \\C' }, light)["--font-ui"]).toBe(
      '"A \\"B\\" \\\\C", sans-serif',
    );
  });
});
