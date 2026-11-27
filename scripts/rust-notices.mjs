#!/usr/bin/env node
// The licence notices of the Rust crates Aximar ships, in
// public/licenses/third-party-rust.html.  See docs/licences.md.
//
//   npm run licences:rust            regenerate the file
//   npm run licences:rust -- --check fail if the file is out of date
//
// Runs cargo-about (0.9.3; `cargo install cargo-about --locked --features
// cli`) once for each shipped binary: the desktop app, maxima-lsp,
// maxima-dap and aximar-mcp.  about.toml holds the accepted licences, and
// cargo-about fails on a crate outside them.  This script merges the four
// results, leaves out Aximar's own crates, and writes one page: each
// licence text once, with the crates that use it and a link to each
// crate's source, which MPL-2.0 asks for.

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const OUTPUT = "public/licenses/third-party-rust.html";
export const ROOTS = [
  "src-tauri/Cargo.toml",
  "crates/maxima-lsp/Cargo.toml",
  "crates/maxima-dap/Cargo.toml",
  "crates/aximar-mcp/Cargo.toml",
];

/** cargo-about's JSON for one binary */
function aboutJson(manifest) {
  const dir = mkdtempSync(join(tmpdir(), "aximar-about-"));
  const out = join(dir, "about.json");
  try {
    try {
      execFileSync(
        "cargo",
        ["about", "generate", "--manifest-path", manifest, "--config", "about.toml", "--locked", "--fail", "--format", "json", "--output-file", out],
        { cwd: ROOT, stdio: ["ignore", "ignore", "pipe"] },
      );
    } catch (e) {
      // cargo-about's report, such as a crate under a licence not accepted
      process.stderr.write(e.stderr ?? "");
      throw new Error(`cargo about failed for ${manifest}`);
    }
    return JSON.parse(readFileSync(out, "utf8"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** A crate from a registry or git, not one of Aximar's own */
const thirdParty = (krate) => krate.source != null;

/**
 * The licences across several cargo-about results: one entry per distinct
 * text, with every third-party crate that uses it
 */
export function merge(results) {
  const byText = new Map();
  for (const result of results) {
    for (const licence of result.licenses) {
      const crates = licence.used_by.map((u) => u.crate).filter(thirdParty);
      if (crates.length === 0) continue;
      const key = `${licence.id}\n${licence.text.trim()}`;
      const entry = byText.get(key) ?? { id: licence.id, name: licence.name, text: licence.text.trim(), crates: new Map() };
      for (const c of crates) {
        entry.crates.set(`${c.name}@${c.version}`, { name: c.name, version: c.version, repository: c.repository ?? null });
      }
      byText.set(key, entry);
    }
  }
  const byName = (a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version, undefined, { numeric: true });
  return [...byText.values()]
    .map((e) => ({ ...e, crates: [...e.crates.values()].sort(byName) }))
    .sort((a, b) => a.id.localeCompare(b.id) || byName(a.crates[0], b.crates[0]));
}

const escape = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** The page */
export function render(licences) {
  const crates = new Set(licences.flatMap((l) => l.crates.map((c) => `${c.name}@${c.version}`)));
  const perId = new Map();
  for (const l of licences) {
    const names = perId.get(l.id) ?? { name: l.name, crates: new Set() };
    for (const c of l.crates) names.crates.add(`${c.name}@${c.version}`);
    perId.set(l.id, names);
  }
  const overview = [...perId]
    .map(([id, { name, crates: set }]) => `  <li>${escape(name)} (${escape(id)}): ${set.size} ${set.size === 1 ? "crate" : "crates"}</li>`)
    .join("\n");
  const sections = licences
    .map((l, i) => {
      const items = l.crates
        .map((c) => {
          const source = `https://docs.rs/crate/${encodeURIComponent(c.name)}/${encodeURIComponent(c.version)}/source/`;
          const repo = c.repository ? `, <a href="${escape(c.repository)}">repository</a>` : "";
          return `  <li>${escape(c.name)} ${escape(c.version)}: <a href="${source}">source</a>${repo}</li>`;
        })
        .join("\n");
      return `<section id="licence-${i + 1}">\n<h2>${escape(l.name)} (${escape(l.id)})</h2>\n<ul>\n${items}\n</ul>\n<pre>${escape(l.text)}</pre>\n</section>`;
    })
    .join("\n\n");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="aximar-notices" content="rust">
<title>Rust crates: licence notices</title>
<style>
  :root { color-scheme: light dark; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
  body { max-width: 860px; margin: 24px auto; padding: 0 16px; line-height: 1.5; }
  pre { white-space: pre-wrap; font-size: 12px; }
  li { margin: 2px 0; }
</style>
</head>
<body>
<h1>Rust crates: licence notices</h1>
<p>Aximar's desktop app and its tools (maxima-lsp, maxima-dap, aximar-mcp) link the ${crates.size} Rust crates below, on the platforms Aximar builds for. Each licence text appears once, with the crates Aximar uses under it, and each crate links its source.</p>
<h2>Licences</h2>
<ul>
${overview}
</ul>

${sections}
</body>
</html>
`;
}

function main(args) {
  const page = render(merge(ROOTS.map(aboutJson)));
  const path = join(ROOT, OUTPUT);
  if (args.includes("--check")) {
    let current = "";
    try {
      current = readFileSync(path, "utf8");
    } catch {
      // missing counts as out of date
    }
    if (current !== page) {
      console.error(`${OUTPUT} is out of date: run npm run licences:rust and commit it`);
      return 1;
    }
    console.log(`${OUTPUT} is up to date`);
    return 0;
  }
  writeFileSync(path, page);
  console.log(`Wrote ${OUTPUT}`);
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = main(process.argv.slice(2));
}
