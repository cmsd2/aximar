// @vitest-environment happy-dom
// A failed cell's output: the cell's bracket or border shows it failed, and
// in the output only a small icon is red
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "@lingui/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { i18n } from "../lib/i18n";
import { EnhancedErrorOutput } from "./EnhancedErrorOutput";
import type { ErrorInfo } from "../types/maxima";

i18n.activate("en");

const divisionByZero: ErrorInfo = {
  id: "error.division_by_zero",
  params: {},
  messages: {},
  title: "Division by Zero",
  explanation: "The expression involves division by zero or raising zero to a negative power, which is undefined.",
  suggestion: "Check your denominator or exponent for cases where the value might be zero.",
  example: null,
  did_you_mean: [],
  correct_signatures: [],
};

function render(errorInfo: ErrorInfo | null, error = "expt: undefined: 0 to a negative exponent."): HTMLElement {
  const div = document.createElement("div");
  div.innerHTML = renderToStaticMarkup(
    <I18nProvider i18n={i18n}>
      <EnhancedErrorOutput error={error} errorInfo={errorInfo} cellId="c1" />
    </I18nProvider>,
  );
  return div;
}

describe("an explained error", () => {
  it("puts the error icon before a plain title", () => {
    const title = render(divisionByZero).querySelector(".enhanced-error-title")!;
    expect(title.querySelector("svg.error-icon")).not.toBeNull();
    expect(title.textContent).toBe("Division by Zero");
  });

  it("puts a lightbulb before the suggestion", () => {
    const suggestion = render(divisionByZero).querySelector(".enhanced-error-suggestion")!;
    expect(suggestion.querySelector("svg.enhanced-error-bulb")).not.toBeNull();
    expect(suggestion.textContent).toContain("Check your denominator");
  });

  it("offers Maxima's own text", () => {
    expect(render(divisionByZero).querySelector(".enhanced-error-toggle")?.textContent).toBe("Show Maxima's error");
  });
});

describe("an error Aximar can't explain", () => {
  it("shows Maxima's text after the same icon", () => {
    const out = render(null, "frobnicate: something went wrong.").querySelector(".error-output")!;
    expect(out.querySelector("svg.error-icon")).not.toBeNull();
    expect(out.querySelector("pre")?.textContent).toBe("frobnicate: something went wrong.");
  });
});

describe("the error styles", () => {
  // The tests run from the project's root; happy-dom's import.meta.url isn't a file
  const css = readFileSync(resolve(process.cwd(), "src/styles/global.css"), "utf8");

  /** The declarations of each rule whose selector names one of `classes` */
  function rules(classes: string[]): string[] {
    return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)]
      .filter(([, selector]) => classes.some((c) => selector.split(",").some((s) => s.trim().endsWith(c) || s.includes(`${c} `))))
      .map(([, selector, body]) => `${selector.trim()} { ${body.trim()} }`);
  }

  it("leave the error colour to the icon", () => {
    const output = rules([".error-output", ".error-output pre", ".enhanced-error", ".enhanced-error-title", ".enhanced-error-explanation", ".enhanced-error-suggestion"]);
    expect(output.length).toBeGreaterThan(4);
    for (const rule of output) expect(rule).not.toMatch(/--error-(color|bg)/);
    expect(rules([".error-icon"]).join(" ")).toContain("var(--error-color)");
  });
});
