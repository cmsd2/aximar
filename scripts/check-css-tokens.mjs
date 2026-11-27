#!/usr/bin/env node
// Keeps new styles on the design tokens.  See docs/design-library.md.
//
//   node scripts/check-css-tokens.mjs           check
//   node scripts/check-css-tokens.mjs --update  record lower counts
//
// Reads each stylesheet under src/ except src/styles/tokens.css, and finds
// the declarations that write a raw value: a colour (#hex, rgb(), hsl()),
// a font size in px, or a corner radius in px.  The baseline,
// scripts/css-tokens-baseline.json, records each such declaration and how
// often it appears.  A declaration not in the baseline, or appearing more
// often, fails the check, with the token to use when one has that value.
// So does a declaration that appears less often than its entry says,
// until --update records the lower count: the baseline only goes down,
// and never by more than the tree has.  --update never adds or raises an
// entry.
//
// It also fails on an import of @tabler/icons-react outside
// src/components/ui/icons.ts, which records each icon the app uses.

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const TOKENS_FILE = "src/styles/tokens.css";
const ICONS_FILE = "src/components/ui/icons.ts";
const BASELINE_FILE = "scripts/css-tokens-baseline.json";

const HEX = /#[0-9a-fA-F]{3,8}\b/;
const FUNCTION_COLOUR = /\b(?:rgba?|hsla?)\(/;
const PX = /(?<![\w.-])(\d*\.?\d+)px\b/;

/** Comments blanked out, keeping line breaks so line numbers hold */
export function stripComments(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, " "));
}

/** Each declaration in a stylesheet: its property, value and line */
export function declarations(css) {
  const text = stripComments(css);
  const out = [];
  const pattern = /([a-z-]+)\s*:\s*([^;{}]+?)\s*(?=[;}])/gi;
  for (const m of text.matchAll(pattern)) {
    // A selector such as `a:hover {` never ends in ; or }, so it can't match
    const line = text.slice(0, m.index).split("\n").length;
    out.push({ property: m[1].toLowerCase(), value: m[2].replace(/\s+/g, " "), line });
  }
  return out;
}

/** The kind of raw value a declaration writes, or null */
export function rawKind({ property, value }) {
  const bare = value.replace(/url\([^)]*\)/g, "");
  if (HEX.test(bare) || FUNCTION_COLOUR.test(bare)) return "colour";
  if (property === "font-size" && PX.test(value)) return "font-size";
  if (/^border(-[a-z]+)*-radius$/.test(property)) {
    const px = [...value.matchAll(new RegExp(PX, "g"))].filter((m) => Number(m[1]) !== 0);
    if (px.length > 0) return "radius";
  }
  return null;
}

/** A declaration as the baseline keys it */
export function keyOf({ property, value }) {
  return `${property}: ${value}`;
}

/** The raw declarations in each stylesheet: { file: [{ key, kind, line }] } */
export function scan(files) {
  const found = {};
  for (const [file, css] of Object.entries(files)) {
    found[file] = declarations(css)
      .map((d) => ({ ...d, kind: rawKind(d) }))
      .filter((d) => d.kind)
      .map((d) => ({ key: keyOf(d), kind: d.kind, line: d.line, value: d.value }));
  }
  return found;
}

function countKeys(entries) {
  const counts = {};
  for (const { key } of entries) counts[key] = (counts[key] ?? 0) + 1;
  return counts;
}

/** Light-theme token values, for suggesting a token in place of a raw value */
export function tokenIndex(tokensCss) {
  const root = stripComments(tokensCss).match(/:root\s*{([^}]*)}/);
  const index = new Map();
  for (const m of (root?.[1] ?? "").matchAll(/--([a-z0-9-]+):\s*([^;]+);/g)) {
    const value = normaliseColour(m[2].trim());
    if (!index.has(value)) index.set(value, `--${m[1]}`);
  }
  return index;
}

function normaliseColour(value) {
  const short = value.match(/^#([0-9a-fA-F])([0-9a-fA-F])([0-9a-fA-F])$/);
  const long = short ? `#${short[1]}${short[1]}${short[2]}${short[2]}${short[3]}${short[3]}` : value;
  return long.toLowerCase();
}

/** The tokens whose value appears in a raw value */
export function suggest(value, index) {
  const names = [];
  for (const m of value.matchAll(new RegExp(`${HEX.source}|${PX.source}`, "g"))) {
    const name = index.get(normaliseColour(m[0]));
    if (name && !names.includes(name)) names.push(name);
  }
  return names;
}

/**
 * Compare what was found with the baseline.  Returns the new raw
 * declarations (failures) and the baseline entries that went down.
 */
export function compare(found, baseline) {
  const failures = [];
  const lowered = [];
  for (const [file, entries] of Object.entries(found)) {
    const counts = countKeys(entries);
    const allowed = baseline[file] ?? {};
    for (const [key, count] of Object.entries(counts)) {
      const limit = allowed[key] ?? 0;
      if (count > limit) {
        for (const e of entries.filter((x) => x.key === key)) failures.push({ file, ...e, count, limit });
      }
    }
    for (const [key, limit] of Object.entries(allowed)) {
      const count = counts[key] ?? 0;
      if (count < limit) lowered.push({ file, key, count, limit });
    }
  }
  for (const [file, allowed] of Object.entries(baseline)) {
    if (found[file]) continue;
    for (const [key, limit] of Object.entries(allowed)) lowered.push({ file, key, count: 0, limit });
  }
  return { failures, lowered };
}

/** The baseline with lower counts recorded; never adds or raises an entry */
export function updated(found, baseline) {
  const next = {};
  for (const [file, allowed] of Object.entries(baseline)) {
    const counts = countKeys(found[file] ?? []);
    const kept = {};
    for (const [key, limit] of Object.entries(allowed).sort(([a], [b]) => a.localeCompare(b))) {
      const count = Math.min(limit, counts[key] ?? 0);
      if (count > 0) kept[key] = count;
    }
    if (Object.keys(kept).length > 0) next[file] = kept;
  }
  return next;
}

/** The baseline for a tree that has none yet: everything found now */
export function initial(found) {
  const out = {};
  for (const [file, entries] of Object.entries(found)) {
    const counts = countKeys(entries);
    if (Object.keys(counts).length === 0) continue;
    out[file] = Object.fromEntries(Object.entries(counts).sort(([a], [b]) => a.localeCompare(b)));
  }
  return out;
}

/** Files other than icons.ts that import @tabler/icons-react */
export function directIconImports(sources) {
  return Object.entries(sources)
    .filter(([file, text]) => file !== ICONS_FILE && /from\s+["']@tabler\/icons-react["']/.test(text))
    .map(([file]) => file);
}

function walk(dir, test) {
  const out = [];
  for (const entry of readdirSync(join(ROOT, dir), { withFileTypes: true })) {
    const path = `${dir}/${entry.name}`;
    if (entry.isDirectory()) out.push(...walk(path, test));
    else if (test(entry.name)) out.push(path);
  }
  return out;
}

function read(paths) {
  return Object.fromEntries(paths.map((p) => [p, readFileSync(join(ROOT, p), "utf8")]));
}

function main(args) {
  const stylesheets = read(walk("src", (n) => n.endsWith(".css")).filter((p) => p !== TOKENS_FILE));
  const found = scan(stylesheets);
  const index = tokenIndex(readFileSync(join(ROOT, TOKENS_FILE), "utf8"));
  const baselinePath = join(ROOT, BASELINE_FILE);
  let baseline;
  try {
    baseline = JSON.parse(readFileSync(baselinePath, "utf8"));
  } catch {
    if (!args.includes("--init")) {
      console.error(`${BASELINE_FILE} is missing; --init records the tree as it is`);
      return 1;
    }
    baseline = initial(found);
    writeFileSync(baselinePath, JSON.stringify(baseline, null, 2) + "\n");
    console.log(`Recorded ${BASELINE_FILE}`);
  }

  const { failures, lowered } = compare(found, baseline);
  const icons = directIconImports(read(walk("src", (n) => /\.tsx?$/.test(n))));
  let status = 0;

  // A declaration new to the baseline: each line.  One the baseline holds
  // fewer times: one line, since any of its places may be the new one.
  const reported = new Set();
  for (const f of failures) {
    const names = suggest(f.value, index);
    const hint = names.length ? `; use ${names.join(" or ")}` : "; add a token, or use one";
    status = 1;
    if (f.limit === 0) {
      console.error(`${f.file}:${f.line}: raw ${f.kind} "${f.key}"${hint}`);
    } else if (!reported.has(`${f.file} ${f.key}`)) {
      reported.add(`${f.file} ${f.key}`);
      console.error(`${f.file}: raw ${f.kind} "${f.key}" appears ${f.count} times, and the baseline allows ${f.limit}${hint}`);
    }
  }
  for (const file of icons) {
    console.error(`${file}: imports @tabler/icons-react directly; import the icon from ${ICONS_FILE}, adding it there`);
    status = 1;
  }

  if (lowered.length > 0) {
    if (args.includes("--update")) {
      writeFileSync(baselinePath, JSON.stringify(updated(found, baseline), null, 2) + "\n");
      console.log(`Lowered ${lowered.length} entries in ${BASELINE_FILE}`);
    } else {
      for (const l of lowered) console.error(`${l.file}: "${l.key}" down from ${l.limit} to ${l.count}`);
      console.error(`Run npm run check:tokens -- --update to record the lower counts`);
      status = 1;
    }
  }

  const total = Object.values(found).reduce((n, e) => n + e.length, 0);
  if (status === 0) console.log(`${total} raw values, none new`);
  return status;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.argv.slice(2));
}

