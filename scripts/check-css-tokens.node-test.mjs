// node --test scripts/check-css-tokens.node-test.mjs  (named so vitest skips it)
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  compare,
  declarations,
  directIconImports,
  initial,
  scan,
  suggest,
  tokenIndex,
  updated,
} from "./check-css-tokens.mjs";

const FIXTURE = `
/* A comment with color: #123456 and font-size: 99px */
.a { color: #333; background: var(--bg-primary); }
.b:hover { border: 1px solid rgba(0, 0, 0, 0.1); }
.c { color: hsl(200, 50%, 40%); fill: url(#gradient); }
.d { font-size: 12px; border-radius: 6px; }
.e { border-radius: 50%; border-top-left-radius: 0; }
.f { font-size: var(--text-sm); border-radius: var(--radius-md); }
.g { accent-color: var(--accent-color, #4a9eff); }
`;

const TOKENS = `
:root {
  --text-primary: #333333;
  --text-sm: 12px;
  --radius-md: 6px;
}
`;

const kinds = (css) =>
  scan({ "a.css": css })["a.css"].map(({ kind, key }) => `${kind} ${key}`);

test("counts_hex_rgb_and_hsl", () => {
  assert.deepEqual(
    kinds(FIXTURE).filter((k) => k.startsWith("colour")),
    [
      "colour color: #333",
      "colour border: 1px solid rgba(0, 0, 0, 0.1)",
      "colour color: hsl(200, 50%, 40%)",
      "colour accent-color: var(--accent-color, #4a9eff)",
    ],
  );
});

test("ignores_comments_and_url_fragments", () => {
  const found = kinds(FIXTURE).join("\n");
  assert.ok(!found.includes("#123456"));
  assert.ok(!found.includes("99px"));
  assert.ok(!found.includes("url(#gradient)"));
});

test("ignores_percent_and_zero_radii", () => {
  assert.deepEqual(
    kinds(FIXTURE).filter((k) => k.startsWith("radius")),
    ["radius border-radius: 6px"],
  );
});

test("counts_px_font_sizes_but_not_tokens", () => {
  assert.deepEqual(
    kinds(FIXTURE).filter((k) => k.startsWith("font-size")),
    ["font-size font-size: 12px"],
  );
});

test("gives_each_declaration_its_line", () => {
  const lines = declarations(FIXTURE).filter((d) => d.value === "#333").map((d) => d.line);
  assert.deepEqual(lines, [3]);
});

test("fails_above_the_baseline", () => {
  const baseline = { "a.css": { "color: #333": 1 } };
  const found = scan({ "a.css": ".a { color: #333; } .b { color: #333; } .c { color: #999; }" });
  const { failures } = compare(found, baseline);
  assert.deepEqual(
    failures.map((f) => f.key),
    ["color: #333", "color: #333", "color: #999"],
  );
});

test("asks_to_record_a_lower_count", () => {
  const baseline = { "a.css": { "color: #333": 2, "color: #999": 1 } };
  const found = scan({ "a.css": ".a { color: #333; }" });
  const { failures, lowered } = compare(found, baseline);
  assert.deepEqual(failures, []);
  assert.deepEqual(
    lowered.map(({ key, count, limit }) => `${key} ${limit}->${count}`),
    ["color: #333 2->1", "color: #999 1->0"],
  );
});

test("update_never_raises_a_count", () => {
  const baseline = { "a.css": { "color: #333": 1, "color: #999": 2 } };
  const found = scan({ "a.css": ".a { color: #333; color: #333; color: #fff; } .b { color: #999; }" });
  assert.deepEqual(updated(found, baseline), { "a.css": { "color: #333": 1, "color: #999": 1 } });
});

test("a_file_with_no_raw_values_leaves_the_baseline", () => {
  const baseline = { "a.css": { "color: #333": 1 } };
  assert.deepEqual(updated(scan({ "a.css": ".a { color: var(--x); }" }), baseline), {});
  assert.deepEqual(initial(scan({ "b.css": ".b { color: var(--x); }" })), {});
});

test("names_the_matching_token", () => {
  const index = tokenIndex(TOKENS);
  assert.deepEqual(suggest("#333", index), ["--text-primary"]);
  assert.deepEqual(suggest("12px", index), ["--text-sm"]);
  assert.deepEqual(suggest("6px", index), ["--radius-md"]);
  assert.deepEqual(suggest("#777", index), []);
});

test("flags_a_direct_icon_import", () => {
  const files = {
    "src/components/ui/icons.ts": 'export { IconX } from "@tabler/icons-react";',
    "src/components/A.tsx": 'import { IconTrash } from "@tabler/icons-react";',
    "src/components/B.tsx": 'import { IconX } from "./ui/icons";',
  };
  assert.deepEqual(directIconImports(files), ["src/components/A.tsx"]);
});
