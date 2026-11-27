import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { CONTRAST_PAIRS } from "./contrast-pairs";
import { contrast, isColour, parseTokens } from "./tokens";

const css = readFileSync(new URL("./tokens.css", import.meta.url), "utf8");
const tokens = parseTokens(css);
const colourNames = [...tokens.light].filter(([, v]) => isColour(v)).map(([k]) => k);

type Themes = Record<"light" | "paper" | "dark", Map<string, string>>;

/** The pairs below 4.5:1 in a theme, as readable lines */
function failures(themes: Themes): string[] {
  const out: string[] = [];
  for (const pair of CONTRAST_PAIRS.filter((p) => !p.exempt)) {
    for (const [theme, map] of Object.entries(themes)) {
      const ratio = contrast(map.get(pair.text)!, map.get(pair.background)!);
      if (ratio < 4.5) out.push(`--${pair.text} on --${pair.background}, ${theme}: ${ratio.toFixed(2)}:1`);
    }
  }
  return out;
}

describe("the design tokens", () => {
  it("give every colour a dark value", () => {
    expect(colourNames.length).toBeGreaterThan(10);
    for (const name of colourNames) {
      expect(tokens.dark.has(name), `--${name} has no dark value`).toBe(true);
    }
  });

  it("give every colour a paper value", () => {
    for (const name of colourNames) {
      expect(tokens.paper.has(name), `--${name} has no paper value`).toBe(true);
    }
  });

  it("keep the two dark blocks the same", () => {
    expect([...tokens.darkAuto]).toEqual([...tokens.dark]);
  });

  it("pair only colours that exist", () => {
    for (const { text, background } of CONTRAST_PAIRS) {
      expect(colourNames, `--${text}`).toContain(text);
      expect(colourNames, `--${background}`).toContain(background);
    }
  });

  it("give text at least 4.5:1 in every theme", () => {
    expect(failures({ light: tokens.light, paper: tokens.paper, dark: tokens.dark })).toEqual([]);
  });

  it("print text at 4.5:1 on paper", () => {
    const white = "#ffffff";
    const low = [...tokens.print]
      .filter(([name, value]) => isColour(value) && !name.startsWith("bg-") && name !== "border-color" && name !== "text-placeholder")
      .map(([name, value]) => [name, contrast(value, white)] as const)
      .filter(([, ratio]) => ratio < 4.5);
    expect(low).toEqual([]);
  });

  it("fail a pair below 4.5:1", () => {
    // The light success text as it was before the settings-window change
    const light = new Map(tokens.light).set("success-text", "#2d8a4e");
    expect(failures({ light, paper: tokens.paper, dark: tokens.dark })).toContain("--success-text on --success-bg, light: 3.95:1");
  });

  it("measure contrast as WCAG does", () => {
    expect(contrast("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrast("#ffffff", "#ffffff")).toBe(1);
    expect(contrast("#767676", "#ffffff")).toBeCloseTo(4.54, 2);
  });
});
