import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import * as icons from "../../components/ui/icons";

const source = readFileSync(new URL("../../components/ui/icons.ts", import.meta.url), "utf8");

describe("icons.ts", () => {
  it("gives every icon a use note, which the icons story shows", () => {
    const exported = Object.keys(icons).filter((k) => k.startsWith("Icon")).sort();
    const noted = [...source.matchAll(/\/\*\*\s*(.+?)\s*\*\/\s*\nexport \{ (Icon\w+) \}/g)].map((m) => m[2]).sort();
    expect(noted).toEqual(exported);
  });
});
