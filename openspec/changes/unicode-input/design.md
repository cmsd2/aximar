## Context

`evaluation::prepare_input` translates Unicode, then rewrites label references, for cells and expressions from both front ends (group 6 of `unify-gui-mcp-operations`). The translation is `maxima::unicode::unicode_to_maxima`: a fixed table applied with string replacement to everything outside `"` pairs, then passes for superscripts and subscripts. `docs/unicode-input.md` lists every character this change handles, with the outcome, display and hint; it is the reference for users and the source of the test cases.

Maxima 5.50 accepts any Unicode letter as a name character, including Greek, letterlike symbols (`ℝ`, `ⅈ`), mathematical alphanumerics (`𝑥`, `𝐱`) and modifier letters (`ᵀ`, `ₙ`). It rejects symbols (`−`, `≈`, `√`, `°`), smart quotes, non-breaking spaces and emoji with "is not an infix operator".

## Goals / Non-Goals

**Goals:** a defined outcome for every character a user plausibly types or pastes; no silent change of meaning; errors that say what to type; display that matches what was typed.

**Non-Goals:** see the proposal.

## Decisions

### 1. Translation runs on code spans only, in three passes

`unicode_to_maxima(input) -> Result<String, UnsupportedCharacter>` splits the input with `maxima::code_spans::spans`, which knows `\"` escapes and nested comments, and copies text spans unchanged. Each code span goes through:

1. **Normalise characters.** NFC, then a per-character map: spaces to a space, zero-width characters removed, line and paragraph separators to newlines, full-width forms and ligatures to ASCII, variant letters to their main letter (`ϕ` to `φ`, `µ` to `μ`, `∆` to `Δ`), mathematical italic letters to plain letters (`𝑥` to `x`, `𝛼` to `α`, `ℎ` to `h`). After this pass, inside names too, `xϕ` and `xφ` are the same name.
2. **Scan tokens.** A name token is a run of letters, ASCII digits, `_` and `%`, not counting superscript and subscript characters. A token that is one translatable letter, optionally followed by digits or by `_` and digits, becomes its ASCII name (`α1` to `alpha1`). `Γ` followed by `(` (after optional spaces) becomes `gamma`. `π`, `ⅈ`, `ⅇ` and `ℯ` are constants only as a whole token. Any other token stays as typed, after checking that it holds no rejected letter. A token that starts with a digit and holds a translatable letter (`2π`) is rejected with the hint `2*π`.
3. **Symbols.** Outside tokens: operator forms map to ASCII operators, `∧ ∨ ¬` to `and`, `or`, `not` with spaces, superscript runs to `^(...)`, subscript runs to `[...]`, `√` to `sqrt` as below. Anything else non-ASCII is rejected: with a specific hint when the table has one, otherwise with a general message.

*Alternative:* NFKC normalisation instead of a table. It maps `²` to `2` and `½` to `1⁄2`, losing meaning the table keeps, and maps `ℝ` to `R` silently.

### 2. A Greek letter inside a longer name stays as typed

The old translator replaced every occurrence, so `Δx` became the name `Deltax` (displayed as that word) and `xπ` became `x%pi`. Keeping the letter makes the name display as typed. The cost: `Δx` and `Deltax` are different names, as `Δx` and `Dx` are.

### 3. Rejections are errors from `prepare_input`

`prepare_input` returns `InputError`, which holds the message for either a rejected character or a label with no value. Callers already show `UnresolvedLabel` as the cell's error, or as the expression's error; they show `InputError` the same way. The message names the character, its code point, the line when the input has several, and the hint:

> `≈` (U+2248) on line 2 can't be used in Maxima code: Maxima has no approximately-equal operator. To compare with a tolerance, write `abs(a - b) < 1e-6`.

### 4. `√` takes a bracket, a number or a name

`√(` becomes `sqrt(`. `√` followed by a number (digits with an optional decimal part) or a name token becomes `sqrt(...)` around it, unless the next character is a superscript, `^`, `(`, `[` or a name character; then, and before anything else, `√` is rejected with the hint `sqrt(...)`. So `√2`, `√x` and `√α` work, and `√x²` and `√2x`, whose reading differs between readers, fail.

### 5. Display

Maxima's default TeX already gives Greek names their letters, except `theta` (`\vartheta`). Aximar's `texput` table shrinks to `theta`, `hbar` (`\hbar`) and `ell` (`\ell`). Dropping the `epsilon` and `phi` overrides lets Maxima's `\varepsilon` and `\varphi` through, which match the glyphs of `ε` and `φ`. Names that pass through (`Δx`, `é`) display as typed; KaTeX renders them.

### 6. One table, tested row by row

The table lives in `maxima/unicode.rs`. Each row of `docs/unicode-input.md` has a test: input, expected output or expected rejection hint. A test reads `src/lib/math-symbols.ts` and checks each symbol translates to what the list says, and another runs every code cell of the built-in templates and the demo notebook through the table. The frontend's `unicodeToMaxima` and its helpers go. `MATH_SYMBOLS`, which feeds the command palette and completions, keeps only characters the table maps.

## What stays in core and what the adapters keep

All translation and rejection is in aximar-core. MCP and Tauri only display `InputError`'s message, as they display label errors today.

## Risks / Trade-offs

- [Notebooks written against the old behaviour change meaning] → The cases are narrow: a Greek letter inside a longer name, variant letters used as separate names, lookalike capitals, Unicode in comments. The changelog lists them. Most turn into errors with a hint, not silent changes; the exception is a Greek letter inside a longer name, which now stays as typed.
- [A friendly mapping someone wants is rejected] → Rejections can become mappings later without breaking notebooks.
- [The table misses a character] → Unlisted letters pass through, as before, and unlisted symbols get the general message instead of Maxima's syntax error.
