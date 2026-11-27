// node --test scripts/i18n-backticks.node-test.mjs  (named so vitest skips it)
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { mismatches } from "./i18n-backticks.mjs";

const CATALOG = `msgid ""
msgstr ""
"Language: es\\n"

msgctxt "note.solve_arc_trig.text"
msgid "\`solve\` used inverse trigonometric functions."
msgstr "\`solve\` usó funciones trigonométricas inversas."

msgctxt "error.log_zero.suggestion"
msgid "Check the argument of \`log\` for values that make it 0."
msgstr "Revisa el argumento de \`log para valores que lo hagan 0."

msgctxt "error.empty_list.explanation"
msgid "\`{func}\` was given an empty list."
msgstr ""

#, fuzzy
msgctxt "error.part_index.suggestion"
msgid "Check the index against \`length()\`."
msgstr "Revisa el índice."
`;

test("finds a translation that lost a backtick", () => {
  assert.deepEqual(mismatches(CATALOG), [
    {
      context: "error.log_zero.suggestion",
      english: "Check the argument of `log` for values that make it 0.",
      translation: "Revisa el argumento de `log para valores que lo hagan 0.",
    },
  ]);
});

test("warns on the command line, and still succeeds", () => {
  const dir = mkdtempSync(join(tmpdir(), "backticks-"));
  const po = join(dir, "es.po");
  writeFileSync(po, CATALOG);
  const script = new URL("./i18n-backticks.mjs", import.meta.url).pathname;
  const out = execFileSync("node", [script, po], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  assert.equal(out, "");
  const err = execFileSync("sh", ["-c", `node "${script}" "${po}" 2>&1`], { encoding: "utf8" });
  assert.match(err, /warning: .*es\.po \(error\.log_zero\.suggestion\): backticks: 1 in the translation, 2 in the English/);
});
