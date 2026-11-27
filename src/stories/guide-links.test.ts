import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const root = new URL("../../", import.meta.url);
const guide = readFileSync(new URL("docs/design-library.md", root), "utf8");

/** Storybook's id for a title and an export, as its `toId` makes them */
function sanitize(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9-]/g, "-").replace(/-+/g, "-").replace(/^-|-$/g, "");
}
function storyName(exportName: string): string {
  return exportName.replace(/([a-z0-9])([A-Z])/g, "$1 $2").replace(/([A-Z])([A-Z][a-z])/g, "$1 $2");
}

function storyIds(): Set<string> {
  const ids = new Set<string>();
  const walk = (dir: URL) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      if (entry.isDirectory()) walk(new URL(`${entry.name}/`, dir));
      else if (entry.name.endsWith(".stories.tsx")) {
        const text = readFileSync(new URL(entry.name, dir), "utf8");
        const title = text.match(/title:\s*"([^"]+)"/)?.[1];
        if (!title) continue;
        for (const m of text.matchAll(/^export const (\w+)/gm)) ids.add(`${sanitize(title)}--${sanitize(storyName(m[1]))}`);
      }
    }
  };
  walk(new URL("src/stories/", root));
  return ids;
}

const parts = readdirSync(new URL("src/components/ui/", root))
  .filter((f) => /^[A-Z]\w*\.tsx$/.test(f))
  .map((f) => f.replace(/\.tsx$/, ""));

describe("the design library guide", () => {
  it("links only to stories that exist", () => {
    const ids = storyIds();
    expect(ids).toContain("ui-button--variants");
    const linked = [...guide.matchAll(/\?path=\/story\/([a-z0-9-]+)/g)].map((m) => m[1]);
    expect(linked.length).toBeGreaterThan(5);
    expect(linked.filter((id) => !ids.has(id))).toEqual([]);
  });

  it("names every part in src/components/ui with a link to its story", () => {
    expect(parts.length).toBeGreaterThanOrEqual(8);
    const rows = guide.split("\n").filter((line) => line.startsWith("| `"));
    for (const part of parts) {
      const row = rows.find((r) => r.startsWith(`| \`${part}\``));
      expect(row, `${part} has no row in the Parts table`).toBeTruthy();
      expect(row, `${part}'s row has no story link`).toMatch(/\?path=\/story\//);
    }
  });
});
