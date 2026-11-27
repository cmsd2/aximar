// node --test scripts/rust-notices.node-test.mjs  (named so vitest skips it)
import { test } from "node:test";
import assert from "node:assert/strict";
import { merge, render } from "./rust-notices.mjs";

const registry = "registry+https://github.com/rust-lang/crates.io-index";
const krate = (name, version, source = registry, repository = null) => ({ name, version, source, repository });
const licence = (id, text, crates) => ({ id, name: `${id} licence`, text, used_by: crates.map((c) => ({ crate: c })) });

const app = {
  licenses: [
    licence("MIT", "MIT text", [krate("serde", "1.0.0"), krate("aximar-core", "0.1.0", null)]),
    licence("GPL-3.0-or-later", "GPL text", [krate("aximar", "0.15.2", null)]),
    licence("MPL-2.0", "MPL text", [krate("cssparser", "0.29.6", registry, "https://github.com/servo/rust-cssparser")]),
  ],
};
const tool = {
  licenses: [licence("MIT", "MIT text", [krate("serde", "1.0.0"), krate("anyhow", "1.0.1")])],
};

test("leaves out Aximar's own crates and their licence", () => {
  const merged = merge([app]);
  assert.deepEqual(merged.map((l) => l.id), ["MIT", "MPL-2.0"]);
  assert.deepEqual(merged[0].crates.map((c) => c.name), ["serde"]);
});

test("merges a licence text shared by several binaries, listing each crate once", () => {
  const merged = merge([app, tool]);
  const mit = merged.find((l) => l.id === "MIT");
  assert.deepEqual(mit.crates.map((c) => `${c.name}@${c.version}`), ["anyhow@1.0.1", "serde@1.0.0"]);
});

test("keeps different texts of one licence apart", () => {
  const other = { licenses: [licence("MIT", "Copyright (c) Someone else\nMIT text", [krate("rand", "0.9.0")])] };
  assert.equal(merge([app, other]).filter((l) => l.id === "MIT").length, 2);
});

test("links each crate's source and escapes the texts", () => {
  const page = render(merge([app, { licenses: [licence("MIT", "a <b> & c", [krate("x", "1.0.0")])] }]));
  assert.match(page, /cssparser 0\.29\.6: <a href="https:\/\/docs\.rs\/crate\/cssparser\/0\.29\.6\/source\/">source<\/a>, <a href="https:\/\/github\.com\/servo\/rust-cssparser">repository<\/a>/);
  assert.match(page, /a &lt;b&gt; &amp; c/);
  assert.match(page, /link the 3 Rust crates/);
  assert.doesNotMatch(page, /aximar-core/);
});
