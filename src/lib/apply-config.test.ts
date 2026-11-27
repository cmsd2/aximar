// @vitest-environment happy-dom
import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { applyConfig, type ConfigTargets } from "./apply-config";
import type { AppConfig } from "./config-client";
import { baseConfig } from "../components/settings/settings-test-kit";

const config = baseConfig as unknown as AppConfig;

function targets(): ConfigTargets {
  return { setTheme: vi.fn(), setCellStyle: vi.fn(), setAutocompleteMode: vi.fn(), setVariablesOpen: vi.fn() };
}

describe("applying config to the window", () => {
  it("sets the font sizes and Markdown style", () => {
    applyConfig(
      { font_size: 16, print_font_size: 10, markdown_font: "serif", markdown_indent: "aligned" },
      config,
      targets(),
    );
    const style = document.documentElement.style;
    expect(style.getPropertyValue("--font-size-mono")).toBe("16px");
    expect(style.getPropertyValue("--print-font-size")).toBe("10px");
    expect(style.getPropertyValue("--print-font-size-mono")).toBe("9px");
    expect(style.getPropertyValue("--font-family-markdown")).toContain("Georgia");
    expect(style.getPropertyValue("--markdown-indent")).toBe("var(--gutter-width)");
  });

  it("writes all four print margins when one changes", () => {
    applyConfig({ print_margin_left: 30 }, { ...config, print_margin_left: 30 }, targets());
    expect(document.getElementById("print-margins-style")?.textContent).toBe("@page { margin: 15mm 15mm 15mm 30mm; }");
  });

  it("passes the theme, cell style, argument help and variables panel on", () => {
    const t = targets();
    applyConfig({ theme: "dark", cell_style: "card", autocomplete_mode: "hint", variables_open: true }, config, t);
    expect(t.setTheme).toHaveBeenCalledWith("dark");
    expect(t.setCellStyle).toHaveBeenCalledWith("card");
    expect(document.documentElement.dataset.cellStyle).toBe("card");
    expect(t.setAutocompleteMode).toHaveBeenCalledWith("hint");
    expect(t.setVariablesOpen).toHaveBeenCalledWith(true);
  });

  it("leaves alone what didn't change", () => {
    const t = targets();
    applyConfig({ eval_timeout: 60 }, config, t);
    expect(t.setTheme).not.toHaveBeenCalled();
    expect(t.setVariablesOpen).not.toHaveBeenCalled();
  });
});
