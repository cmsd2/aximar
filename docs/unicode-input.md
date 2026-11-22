# Unicode in Maxima input

Status: **in effect**. The requirements are in `openspec/specs/unicode-input/spec.md`. This is the reference for what Aximar does with each non-ASCII character in Maxima code, from the desktop window and from MCP. Notebooks depend on it, so a mapping, once released, is part of the public API. "Changes from earlier releases" at the end lists what changed when this table took effect.

## Principles

1. **Only code is translated.** Text inside strings and comments stays as typed. The scanner is the one label rewriting uses (`maxima::code_spans`), which knows `\"` escapes and nested comments.
2. **One clear meaning becomes that meaning.** `π` is `%pi`, `−` is `-`, `≤` is `<=`.
3. **A Greek letter that stands alone as a name, or with a number after it (`α1`, `α_1`), becomes the ASCII name** a Maxima user would type, so `α` and `alpha` are the same variable, and notebooks and plain Maxima code agree. Output shows the letter again.
4. **Inside a longer name, a Greek letter stays as typed.** `Δx`, `αβ` and `xπ` are names, displayed as typed. Translating them gave names like `Deltax` and `alphabeta`, which display as those words, and `x%pi`.
5. **Variant forms of one letter are that letter.** `ϕ` and `φ` are both `phi`; `µ` (micro sign) is `mu`.
6. **When the meaning is unclear, reject with a hint.** The cell fails before Maxima runs, with a message that says what to type instead. A rejection can become a mapping in a later release without breaking any notebook; a mapping can't be changed without breaking the notebooks that use it.
7. **Letters with no ASCII form pass through.** Maxima accepts `é`, `ж` and `中` as name characters, and they display as typed.
8. **Invisible characters never change meaning.** Non-breaking spaces are spaces, zero-width characters are removed, and text is normalised to NFC first, so `é` typed as `e` plus a combining accent is the same name as `é`.

A rejected character is reported as the cell's error, like a label with no value:

> `≈` (U+2248 ALMOST EQUAL TO) can't be used in Maxima code: Maxima has no approximately-equal operator. To compare with a tolerance, write `abs(a - b) < 1e-6`.

## How to read the tables

| Outcome | Meaning |
|---|---|
| `alpha` | Becomes this Maxima name, constant or operator |
| unchanged | Sent to Maxima as typed; Maxima treats it as a name character |
| **rejected** | The cell fails before Maxima runs, with the hint shown |
| space / removed | Whitespace handling |

"Shows as" is how the output renders. Maxima's own TeX output gives most Greek names their letters; Aximar adds a `texput` only where Maxima's default differs from the character typed.

"Before" shows what a row did in earlier releases, where that differs (see the last section).

## Greek, lowercase

| Char | Code point | Outcome | Shows as | Notes | Before |
|---|---|---|---|---|---|
| α | U+03B1 | `alpha` | α | | |
| β | U+03B2 | `beta` | β | | |
| γ | U+03B3 | `gamma` | γ | A variable, not Euler's constant (`%gamma`). `γ(5)` calls Maxima's `gamma` function and gives 24. | |
| δ | U+03B4 | `delta` | δ | `δ(x)` is Maxima's Dirac delta (used by `laplace`). | |
| ε | U+03B5 | `epsilon` | ε | | shows ϵ |
| ζ | U+03B6 | `zeta` | ζ | `ζ(2)` is the Riemann zeta function: `π²/6`. | |
| η | U+03B7 | `eta` | η | | |
| θ | U+03B8 | `theta` | θ | Needs `texput`: Maxima's default is ϑ. | |
| ι | U+03B9 | `iota` | ι | | |
| κ | U+03BA | `kappa` | κ | | |
| λ | U+03BB | `lambda` | λ | Works as a variable (`λ: 2` is fine). `λ([x], x^2)` is an anonymous function. | |
| μ | U+03BC | `mu` | μ | | |
| ν | U+03BD | `nu` | ν | | |
| ξ | U+03BE | `xi` | ξ | | |
| ο | U+03BF | **rejected** | | Omicron looks like Latin `o`. Hint: type `o`. | unchanged |
| π | U+03C0 | `%pi` | π | The constant. π can't be a variable. | |
| ρ | U+03C1 | `rho` | ρ | | |
| ς | U+03C2 | `sigma` | σ | Final sigma, a form of σ. | unchanged |
| σ | U+03C3 | `sigma` | σ | | |
| τ | U+03C4 | `tau` | τ | | |
| υ | U+03C5 | `upsilon` | υ | | |
| φ | U+03C6 | `phi` | φ | Not the golden ratio (`%phi`). | shows ϕ |
| χ | U+03C7 | `chi` | χ | | |
| ψ | U+03C8 | `psi` | ψ | `ψ[0](x)` is Maxima's digamma function. | |
| ω | U+03C9 | `omega` | ω | | |

## Greek, uppercase

| Char | Code point | Outcome | Shows as | Notes | Before |
|---|---|---|---|---|---|
| Γ | U+0393 | `Gamma`; `gamma` before `(` | Γ | `Γ(5)` gives 24, Maxima's gamma function, which displays as Γ. `Γ` alone is the variable `Gamma`. `Γ(x) := ...` redefines Maxima's gamma function, as `gamma(x) := ...` does. | `Γ(5)` stays `Gamma(5)` |
| Δ | U+0394 | `Delta` | Δ | | |
| Θ | U+0398 | `Theta` | Θ | | |
| Λ | U+039B | `Lambda` | Λ | | |
| Ξ | U+039E | `Xi` | Ξ | | |
| Π | U+03A0 | `Pi` | Π | A variable, not a product. Hint in docs: `product(f(k), k, 1, n)`. | |
| Σ | U+03A3 | `Sigma` | Σ | A variable, not a sum. Hint in docs: `sum(f(k), k, 1, n)`. | |
| Φ | U+03A6 | `Phi` | Φ | | |
| Ψ | U+03A8 | `Psi` | Ψ | | |
| Ω | U+03A9 | `Omega` | Ω | | |
| Α Β Ε Ζ Η Ι Κ Μ Ν Ο Ρ Τ Υ Χ | U+0391 … U+03A7 | **rejected** | | These look like Latin `A B E Z H I K M N O P T Y X` but would be different names, a source of bugs nobody can see. Hint: type the Latin letter. For capital upsilon in maths, use `ϒ`. | unchanged |

## Greek variants and lookalike symbols

| Char | Code point | Outcome | Shows as | Notes | Before |
|---|---|---|---|---|---|
| ϐ | U+03D0 beta symbol | `beta` | β | | unchanged |
| ϑ | U+03D1 theta symbol | `theta` | θ | | unchanged |
| ϒ | U+03D2 upsilon with hook | `Upsilon` | ϒ | The mathematical capital upsilon. | unchanged |
| ϕ | U+03D5 phi symbol | `phi` | φ | | unchanged |
| ϖ | U+03D6 pi symbol | **rejected** | | Not the constant π, and rarely a variable. Hint: use `π` for the constant, or a name. | unchanged |
| ϰ | U+03F0 kappa symbol | `kappa` | κ | | unchanged |
| ϱ | U+03F1 rho symbol | `rho` | ρ | | unchanged |
| ϴ | U+03F4 capital theta symbol | `Theta` | Θ | | unchanged |
| ϵ | U+03F5 lunate epsilon | `epsilon` | ε | | unchanged |
| µ | U+00B5 micro sign | `mu` | μ | macOS types this for Option-M. | unchanged |
| Ω | U+2126 ohm sign | `Omega` | Ω | | unchanged |
| ∆ | U+2206 increment | `Delta` | Δ | macOS types this for Option-J. | error |
| ∏ | U+220F n-ary product | **rejected** | | Hint: `product(f(k), k, 1, n)`. Use `Π` (U+03A0) for a variable. | error |
| ∑ | U+2211 n-ary summation | **rejected** | | Hint: `sum(f(k), k, 1, n)`. Use `Σ` (U+03A3) for a variable. | error |
| Other Greek letters (with accents, archaic) | | unchanged | as typed | Letters with no Maxima name, such as `ά`. | |

## Mathematical letters

| Char | Code point | Outcome | Shows as | Notes | Before |
|---|---|---|---|---|---|
| ⅈ | U+2148 double-struck italic i | `%i` | i | The imaginary unit. | unchanged |
| ⅇ | U+2147 double-struck italic e | `%e` | e | | unchanged |
| ℯ | U+212F script e | `%e` | e | Unicode's "natural exponent". | unchanged |
| ⅉ | U+2149 double-struck italic j | **rejected** | | Hint: Maxima's imaginary unit is `%i`. | unchanged |
| ℏ | U+210F h-bar | `hbar` | ℏ | Needs `texput` to `\hbar`. | unchanged |
| ℓ | U+2113 script l | `ell` | ℓ | Needs `texput` to `\ell`. | unchanged |
| ℎ | U+210E Planck constant | `h` | h | Mathematical italic h. | unchanged |
| 𝐴 … 𝑧, 𝛢 … 𝜔 | U+1D434 … U+1D467, U+1D6E2 … U+1D71B | the plain letter | as typed | Mathematical italic, as pasted from PDFs and Word. Output is italic anyway, so `𝑥` and `x` look and behave the same. Italic Greek follows the Greek tables. | unchanged |
| Bold, bold italic, script, fraktur, sans-serif and monospace letters and digits | U+1D400 … U+1D7FF (other than italic) | **rejected** | | Styles usually carry meaning, such as bold for vectors, and Maxima names have no style. Hint: name the plain letter, for example `x`, or a name such as `xvec`. | unchanged |
| ℕ ℤ ℚ ℝ ℂ ℙ ℍ, and 𝔸 … 𝕫 | U+2115 etc., U+1D538 … U+1D56B | **rejected** | | Maxima has no number-set values. Hint: `declare(x, real)`, `declare(n, integer)` or `assume(x > 0)`. | unchanged |
| ⅅ ⅆ | U+2145, U+2146 | **rejected** | | Differential d. Hint: `diff(f, x)`, `integrate(f, x)`. | unchanged |

## Operators and relations

| Char | Code point | Outcome | Notes | Before |
|---|---|---|---|---|
| − | U+2212 minus sign | `-` | Typeset minus, common in pasted text. | error |
| × | U+00D7 | `*` | Multiplication. For vectors, `*` is element by element; the cross product is `~` in the `vect` package. | |
| · | U+00B7 middle dot | `*` | | |
| ⋅ | U+22C5 dot operator | `*` | | error |
| ∗ | U+2217 asterisk operator | `*` | | error |
| ÷ | U+00F7 | `/` | | |
| ∕ | U+2215 division slash | `/` | | error |
| ⁄ | U+2044 fraction slash | `/` | | error |
| ≤ ≦ ⩽ | U+2264, U+2266, U+2A7D | `<=` | | `≦ ⩽` error |
| ≥ ≧ ⩾ | U+2265, U+2267, U+2A7E | `>=` | | `≧ ⩾` error |
| ≠ | U+2260 | `#` | Maxima's not-equal. | |
| ∧ | U+2227 | ` and ` | Spaces added: `a∧b` is `a and b`. | error |
| ∨ | U+2228 | ` or ` | | error |
| ¬ | U+00AC | `not ` | | error |
| √ | U+221A | `sqrt(` before `(`; `sqrt(n)` around a number or name | `√(x+1)` is `sqrt(x+1)`; `√2` is `sqrt(2)`; `√x` is `sqrt(x)`. When the number or name is followed by a power, a bracket or another name character (`√x²`, `√x[1]`, `√2x`), or `√` comes before anything else, it is rejected with the hint `sqrt(...)`. | error |
| ∛ ∜ | U+221B, U+221C | **rejected** | Hint: `x^(1/3)`. | error |
| ∞ | U+221E | `inf` | `-∞` is `-inf`, which `limit` accepts. | |
| ± ∓ | U+00B1, U+2213 | **rejected** | Hint: Maxima has no ± operator; write both cases, `[a + b, a - b]`. | error |
| ≈ ≃ ≅ | U+2248, U+2243, U+2245 | **rejected** | Hint: `abs(a - b) < 1e-6`. | error |
| ≡ | U+2261 | **rejected** | Hint: `=` for an equation, `is(equal(a, b))` to test. | error |
| ≔ | U+2254 | **rejected** | Hint: `:` assigns a value, `:=` defines a function. | error |
| → ⇒ ⟶ ⟹ ⇔ | U+2192 etc. | **rejected** | Hint for →: `limit(f, x, a)`. | error |
| ↦ | U+21A6 | **rejected** | Hint: `lambda([x], ...)`. | error |
| ∂ | U+2202 | **rejected** | Hint: `diff(f, x)`. | error |
| ∇ | U+2207 | **rejected** | Hint: `diff` per variable, or `grad` in the `vect` package. | error |
| ∫ ∬ ∮ | U+222B etc. | **rejected** | Hint: `integrate(f, x, a, b)`. | error |
| ∈ ∉ | U+2208, U+2209 | **rejected** | Hint: `elementp(x, S)`, `member(x, L)`. | error |
| ⊂ ⊆ ⊃ ⊇ | U+2282 etc. | **rejected** | Hint: `subsetp(A, B)`. | error |
| ∪ ∩ ∖ | U+222A, U+2229, U+2216 | **rejected** | Hint: `union(A, B)`, `intersection(A, B)`, `setdifference(A, B)`. | error |
| ∅ | U+2205 | **rejected** | Hint: `{}` is the empty set. | error |
| ∀ ∃ | U+2200, U+2203 | **rejected** | | error |
| ′ ″ ‴ | U+2032 … U+2034 | **rejected** | Hint: `diff(f(x), x)`. | error |
| ° | U+00B0 | **rejected** | Hint: Maxima works in radians; write `30*%pi/180`. A conversion may come with a wider design for units. | error |
| º | U+00BA masculine ordinal | **rejected** | Looks like °. Hint: use `°` or `*%pi/180`. | unchanged |
| ½ ⅓ ¼ … | U+00BD etc. | **rejected** | Hint: `1/2`. `3½` has no single reading. | error |
| ‰ | U+2030 | **rejected** | Hint: `/1000`. | error |
| – — ‐ ‑ | U+2013, U+2014, U+2010, U+2011 | **rejected** | Dashes and hyphens, usually from pasted prose. Hint: `-` for minus. | error |
| … | U+2026 | **rejected** | | error |
| • | U+2022 | **rejected** | | error |

## Superscripts and subscripts

| Chars | Outcome | Notes | Before |
|---|---|---|---|
| ⁰ ¹ ² ³ ⁴ ⁵ ⁶ ⁷ ⁸ ⁹ ⁺ ⁻ ⁿ | a run becomes `^(...)` | `x²` is `x^(2)`, `x⁻¹` is `x^(-1)`, `2ⁿ⁺¹` is `2^(n+1)`. | |
| Other superscript letters (ᵀ ⁱ ˣ …) | **rejected** | Hint: `^`; for a transpose, `transpose(M)`. | unchanged |
| ₀ … ₉ | a run becomes `[...]` | `x₁₂` is `x[12]`. | |
| ₐ ₑ ₒ ₓ ₕ ₖ ₗ ₘ ₙ ₚ ₛ ₜ ᵢ ⱼ ᵣ ᵤ ᵥ ₊ ₋ | join the run | `aₙ` is `a[n]`, `xₙ₊₁` is `x[n+1]`. Two subscript letters together are rejected: `xᵢⱼ` could mean `x[i,j]` or `x[ij]`. | unchanged |

## Spaces, invisible characters and quotes

| Chars | Outcome | Notes | Before |
|---|---|---|---|
| U+00A0 no-break space, U+2000 … U+200A, U+202F, U+205F, U+3000 | space | Common in pasted text. | error |
| U+200B zero-width space, U+200C, U+200D, U+2060, U+FEFF, U+00AD soft hyphen | removed | Invisible, so removing them matches what the user sees. | error |
| U+2028, U+2029 | newline | | |
| U+202A … U+202E, U+2066 … U+2069 (bidirectional controls) | **rejected** | They can make code read differently from how it runs. | |
| “ ” „ ‟ « » | **rejected** | Hint: use straight quotes `"` for strings. Inside a string they stay as typed. | error |
| ‘ ’ ‚ ‛ | **rejected** | Hint: Maxima's quote is `'`. | error |
| Full-width forms U+FF01 … U+FF5E (`ｘ` `（` `＋`) | the ASCII character | Typed by CJK input methods. | letters unchanged, punctuation error |
| Ideographic punctuation (`。` `、`) | **rejected** | | error |
| Ligatures U+FB00 … U+FB06 (`ﬁ` `ﬂ`) | the letters | From text copied out of PDFs. | unchanged |

## Everything else

- After NFC normalisation, any other character that Unicode classes as a letter passes through unchanged: accented Latin, Cyrillic, Greek with accents, Hebrew, Arabic, CJK. Maxima accepts them in names, and they display as typed.
- Any other character (emoji, box drawing, other symbols) is **rejected**, naming the character and its code point. Maxima rejects these too, with "is not an infix operator", which doesn't say what's wrong.
- A digit followed directly by a translated letter (`2π`, `3α`) is rejected with the hint `2*π`. Maxima has no implicit multiplication, and its own error would name `%pi`, which the user never typed.

## Output

- TeX output (rendered maths) shows Greek names as their letters, through Maxima's defaults plus `texput` for `theta`, `hbar` and `ell`. Aximar no longer overrides `epsilon` and `phi`: Maxima's defaults, `\varepsilon` and `\varphi`, match the glyphs of `ε` and `φ`.
- Text output (`print`, `string`, error messages, MCP `text_output`) shows Maxima's names: `alpha`, `%pi`.
- Names that pass through display as typed in both.

## Decisions

These were open when the table was drafted:

1. **Γ before `(`** calls Maxima's gamma function, so `Γ(5)` gives 24. `Γ` alone is the variable `Gamma`.
2. **Degrees** are rejected with a hint for now. Turning `30°` into `30*%pi/180` adds a unit conversion to the input, which belongs with a wider design for units.
3. **√** works before a bracket, a number or a name, and is rejected before anything else.
4. **Greek capitals that look like Latin letters**, and omicron, are rejected with a hint to type the Latin letter.

## Changes from earlier releases

- `ε` and `φ` display as typed (they showed `ϵ` and `ϕ`).
- A Greek letter inside a longer name stays as typed. Before, `αβ` became `alphabeta`, `Δx` became `Deltax`, and `xπ` became `x%pi`. Maxima's TeX shows such names literally, so `r_θ` displayed as "r_theta" and now displays as "r_θ"; only a trailing number becomes a subscript (`α_1` shows as α₁).
- Variants (`ϕ ϑ ϵ ϱ ϰ ς ϐ ϴ ϒ µ`, ohm sign, increment) map to their letters; they passed through as different names.
- Omicron and the Greek capitals that look like Latin letters are rejected; they passed through as different names. `Υ` is among them; `ϒ` is the mathematical capital upsilon.
- New mappings: `−` and the other operator forms, `∧ ∨ ¬`, `√`, `Γ(`, `ⅈ ⅇ ℯ`, `ℏ ℓ ℎ`, mathematical italic letters, subscript letters, spaces, full-width forms and ligatures.
- Characters Maxima can't read are reported with a hint instead of Maxima's "is not an infix operator".
- Strings containing `\"` and comments are no longer translated. Before, the translator didn't know escapes, so the rest of a string after `\"` was translated as code, and so was text in comments.

## Implementation notes

- One table in `aximar-core`, used by `evaluation::prepare_input`. A rejection is an error like `UnresolvedLabel`, shown as the cell's error before Maxima runs.
- The unused TypeScript translator (`unicodeToMaxima` in `src/lib/math-symbols.ts`) goes. The command palette and completions keep `MATH_SYMBOLS`, which should list only characters this table maps.
- Each row is a test case, so the table and the code can't drift. Two more tests check that every symbol in the frontend's list translates to what the list says, and that every code cell in the built-in templates and `docs/examples/demo1.macnb` gets past the table.
