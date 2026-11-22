## Why

Aximar translates some Unicode in Maxima code before Maxima sees it: Greek letters become names like `alpha`, `π` becomes `%pi`, superscripts become powers. Notebooks depend on that translation, so it is part of the public API, but nothing defines it beyond a table in the code, and it has gaps:

- Characters outside the table reach Maxima unchanged. Maxima accepts any Unicode letter as a name character, so `ϕ` and `φ` are different variables, and a Greek capital `Α` is a different name from the Latin `A` it looks like.
- Symbols outside the table, such as the typeset minus `−`, a non-breaking space or a smart quote from pasted text, fail in Maxima with "is not an infix operator", which doesn't say what's wrong.
- A Greek letter inside a longer name is translated too, so `Δx` becomes `Deltax` and displays as that word, and `xπ` becomes the name `x%pi`.
- The translator doesn't know `\"` escapes, so it translates part of some strings, and it translates comments.
- `ε` and `φ` display as `ϵ` and `ϕ`, because Aximar overrides Maxima's TeX names for them.
- A second, unused copy of the translator lives in the frontend.

`docs/unicode-input.md` sets out what each character should do. This change makes it the specification and implements it.

## What Changes

- One table in aximar-core decides, for each non-ASCII character in code, whether it becomes a Maxima name, constant or operator, passes through unchanged, or is rejected with a hint. Strings and comments stay as typed.
- A Greek letter that is a whole name, optionally with a number (`α`, `α1`, `α_1`), becomes its ASCII name; inside a longer name it stays as typed.
- Variant forms of a letter (`ϕ`, `ϑ`, `ϵ`, `µ`, `∆`, …) are that letter.
- `Γ` before a bracket calls Maxima's `gamma`. `√` before a bracket, a number or a name becomes `sqrt`.
- New operator forms (`−`, `⋅`, `∗`, `∕`, `≦`, `∧`, `∨`, `¬`, …), constants (`ⅈ`, `ⅇ`, `ℯ`) and names (`ℏ`, `ℓ`).
- Pasted text is cleaned: no-break and other spaces become spaces, zero-width characters are removed, full-width forms and ligatures become ASCII, and code is normalised to NFC.
- Characters with no clear meaning are rejected before Maxima runs, as the cell's error, naming the character, its code point and what to type instead. Lookalike Greek capitals, `°`, `∑`, `ℝ`, `≈` and smart quotes are among them.
- `ε` and `φ` display as typed.
- The unused frontend translator is removed.
- **BREAKING (notebooks):** inputs that relied on the old gaps change meaning or fail: a Greek letter inside a longer name, variant letters used as separate names, lookalike capitals, and Unicode in comments or after `\"` in strings. Each now behaves as the table says.

## Capabilities

### New Capabilities

- `unicode-input`: what Aximar does with each non-ASCII character in Maxima code, from either front end, and how translated names display.

### Modified Capabilities

None. `cell-execution` (in `unify-gui-mcp-operations`) already requires cells and expressions to share one input preparation; this change defines the Unicode step of it.

## Impact

- **Layers:** core (`maxima/unicode.rs`, `evaluation.rs`), MCP and Tauri (the error type of `prepare_input`), frontend (`math-symbols.ts`).
- **Issues:** none filed; the work came out of reviewing group 6 of `unify-gui-mcp-operations`.
- **MCP clients:** inputs with rejected characters fail with a message instead of a Maxima syntax error. The server instructions' advice to write Greek letters still holds.
- **Saved notebooks:** no change to the file format. Notebooks that use the inputs listed under BREAKING behave differently when run.
- **Dependencies:** `unicode-normalization` in aximar-core.

## Non-goals

- Units, including degrees. `°` is rejected with a hint until there is a design for units.
- Unicode in `.mac` files loaded with `load` or `batch`. Maxima reads those itself.
- The language server and `.mac` parser, which see files, not notebook input.
- Translating output back to Unicode in text output (`print`, `string`), which shows Maxima's names.
