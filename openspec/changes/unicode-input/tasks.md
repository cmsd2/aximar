Each numbered group is one pull request. Every group runs `cargo test --workspace`, `cargo test -p aximar-mcp -- --ignored` and `cargo test -p aximar-core --test unicode_input -- --ignored` (with Maxima), and `npm test`.

## 1. Unicode input

- [ ] 1.1 Rewrite `maxima::unicode::unicode_to_maxima` as the three passes in the design, over `code_spans`, returning `Result<String, UnsupportedCharacter>`; add `unicode-normalization`; verify with one unit test per row of `docs/unicode-input.md` (input, expected output or expected hint), plus the scenarios of the `unicode-input` spec that need no Maxima
- [ ] 1.2 Make `prepare_input` return `InputError` for a rejected character or a label with no value, and show it as the cell's or expression's error in `evaluate_cell`, MCP `evaluate_expression` and the Tauri `evaluate_expression`; verify with a core test that a rejected character fails a cell before Maxima runs, and an aximar-mcp test that `∑ k` fails with a hint
- [ ] 1.3 Shrink the `texput` table to `theta`, `hbar` and `ell`; verify with a Maxima-backed core test (`tests/unicode_input.rs`) for the display scenario, and for the scenarios that need Maxima: `α` and `alpha` are one variable, `Γ(5)` is 24, `ϕ` and `φ` are one variable, and a combining accent names the same variable
- [ ] 1.4 Remove `unicodeToMaxima` and its helpers from `src/lib/math-symbols.ts`, and keep only characters the table maps in `MATH_SYMBOLS`; verify `npm test` and that the command palette and completions still list Greek letters
- [ ] 1.5 Update the CHANGELOG (with the breaking cases) and the MCP server instructions if they describe the translation

## 2. Close-out

- [ ] 2.1 Archive this change with `openspec archive unicode-input`, and check that `openspec/specs/` gains `unicode-input`
