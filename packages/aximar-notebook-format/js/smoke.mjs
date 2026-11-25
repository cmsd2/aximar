// Checks the npm build (pkg/, from wasm-pack) against the shared test
// notebooks.  The Rust tests cover the format itself; this checks that the
// WebAssembly build gives JavaScript the same results.
//
// Run: wasm-pack build --release --target nodejs --out-dir pkg -- --features wasm
//      node --test js/smoke.mjs

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { read, write, newId, validId } = require("../pkg/aximar_notebook_format.js");

const fixture = (name) => readFileSync(new URL(`../fixtures/${name}`, import.meta.url), "utf8");

test("writes the same bytes as the Rust build", () => {
  const text = fixture("spec-example.macnb");
  const file = read(text);
  assert.equal(file.idsReplaced, false);
  assert.equal(write(file), text);
});

test("gives plain objects with nulls for missing values", () => {
  const file = read(fixture("vscode-extension-before-s1.macnb"));
  const plot = file.cells.find((c) => c.input === "plot2d(x^2, [x, 0, 1])$");
  assert.equal(plot.cellType, "code");
  assert.match(plot.output.plotSvg, /^<svg|^<\?xml/);
  assert.equal(plot.output.textOutput, "");
  assert.equal(plot.output.latex, null);
  assert.equal(plot.output.stale, true);
  // The old extension's text/plain copy of the LaTeX isn't a one-line form
  const result = file.cells.find((c) => c.input.endsWith("integrate(x, x);"));
  assert.equal(result.output.textPlain, null);
  const failed = file.cells.find((c) => c.input === "1/0;");
  assert.equal(failed.output.isError, true);
  assert.match(failed.output.error, /^expt: undefined/);
  assert.equal(Object.getPrototypeOf(file.metadata), Object.prototype);
});

test("replaces bad IDs and makes valid new ones", () => {
  const file = read(fixture("cell-ids.macnb"));
  assert.equal(file.idsReplaced, true);
  assert.equal(file.cells[0].id, "a1");
  assert.ok(file.cells.every((c) => validId(c.id)));
  const id = newId();
  assert.ok(validId(id) && id.length === 21);
  assert.equal(validId("bad id!"), false);
});

test("writes a notebook built in JavaScript, with optional fields left out", () => {
  const text = write({
    cells: [
      { id: "c1", cellType: "code", input: "x^2/2;", output: { latex: "{{x^2}\\over{2}}", textPlain: "x^2/2", executionCount: 1 } },
      { id: "m1", cellType: "markdown", input: "# Notes" },
    ],
  });
  const json = JSON.parse(text);
  assert.equal(json.nbformat_minor, 5);
  assert.equal(json.metadata.kernelspec.name, "maxima");
  assert.deepEqual(json.cells[0].outputs[0].data, { "text/latex": ["{{x^2}\\over{2}}"], "text/plain": ["x^2/2"] });
  assert.equal(json.cells[1].outputs, undefined);
  // and reads back to the same cells
  const back = read(text);
  assert.equal(back.cells[0].output.textPlain, "x^2/2");
});

test("keeps metadata nulls and unknown outputs", () => {
  const text = `{"nbformat": 4, "nbformat_minor": 5, "metadata": {"x": null}, "cells": [
    {"id": "c", "cell_type": "code", "source": "1;", "metadata": {}, "execution_count": null,
     "outputs": [{"output_type": "future_type", "payload": null}]}]}`;
  const json = JSON.parse(write(read(text)));
  assert.equal(json.metadata.x, null);
  assert.deepEqual(json.cells[0].outputs, [{ output_type: "future_type", payload: null }]);
});

test("refuses files that aren't nbformat 4", () => {
  assert.throws(() => read('{"nbformat": 3, "cells": []}'), /nbformat 3/);
  assert.throws(() => read("not json"), /not valid JSON/);
});
