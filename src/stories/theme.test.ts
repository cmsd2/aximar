// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { applyStoryTheme } from "./theme";

describe("the Storybook theme switch", () => {
  it("sets data-theme from the global", () => {
    applyStoryTheme("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    applyStoryTheme("light");
    expect(document.documentElement.dataset.theme).toBe("light");
  });

  it("falls back to light", () => {
    expect(applyStoryTheme(undefined)).toBe("light");
    expect(applyStoryTheme("sepia")).toBe("light");
  });
});
