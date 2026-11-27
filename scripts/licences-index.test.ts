import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const root = new URL("../", import.meta.url);
const index = readFileSync(new URL("public/licenses/index.html", root), "utf8");
/** Files the build writes into dist/licenses, beside public/licenses */
const BUILT = ["third-party-npm.txt"];

describe("the licences index", () => {
  it("links only files the app ships", () => {
    const hrefs = [...index.matchAll(/href="([^"]+)"/g)].map((m) => m[1]);
    expect(hrefs.length).toBe(4);
    for (const href of hrefs) {
      const there = BUILT.includes(href) || existsSync(new URL(`public/licenses/${href}`, root));
      expect(there, href).toBe(true);
    }
  });

  it("links every licence file in public/licenses", () => {
    for (const file of ["third-party-rust.html", "JetBrainsMono-OFL.txt", "ComputerModern-OFL.txt", ...BUILT]) {
      expect(index).toContain(`href="${file}"`);
    }
  });
});
