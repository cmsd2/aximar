# unicode-input Specification

## Purpose

What Aximar does with non-ASCII characters in Maxima code, for cells and expressions from either front end, and how the names it produces display. `docs/unicode-input.md` lists every character with its outcome; these requirements state the rules it follows.

## ADDED Requirements

### Requirement: Only code is translated

Translation SHALL apply to code only. String literals and comments SHALL reach Maxima as typed, including strings that contain `\"` and nested comments.

#### Scenario: String with an escaped quote

- **WHEN** a cell's input is `print("say \"α\" and β");`
- **THEN** the input sent to Maxima SHALL be unchanged

#### Scenario: Comment

- **WHEN** a cell's input is `/* α is the angle */ α: 1;`
- **THEN** the input sent to Maxima SHALL be `/* α is the angle */ alpha: 1;`

### Requirement: A Greek letter that is a whole name becomes its ASCII name

A Greek letter that makes up a whole name, optionally followed by digits or by `_` and digits, SHALL become the Maxima name for it (`α` to `alpha`, `Δ` to `Delta`), so the letter and the ASCII name are the same variable. `π` SHALL become the constant `%pi`. `Γ` followed by `(` SHALL become `gamma`, Maxima's gamma function.

#### Scenario: Letter and ASCII name are one variable

- **WHEN** one cell runs `α: 2$` and the next runs `alpha + 1;`
- **THEN** the result SHALL be `3` (needs Maxima)

#### Scenario: Letter with a number

- **WHEN** a cell's input is `α1 + α_2;`
- **THEN** the input sent to Maxima SHALL be `alpha1 + alpha_2;`

#### Scenario: Gamma function

- **WHEN** a cell runs `Γ(5);`
- **THEN** the result SHALL be `24` (needs Maxima)

#### Scenario: Pi

- **WHEN** a cell's input is `sin(π/2);`
- **THEN** the input sent to Maxima SHALL be `sin(%pi/2);`

### Requirement: A Greek letter inside a longer name stays as typed

A Greek letter that is part of a longer name, other than the forms above, SHALL stay as typed, so the name displays as typed.

#### Scenario: Name with a Greek letter

- **WHEN** a cell's input is `Δx: 1$ xπ + αβ;`
- **THEN** the input sent to Maxima SHALL be `Δx: 1$ xπ + αβ;`

### Requirement: Variant forms of a letter are that letter

A variant form of a letter (such as `ϕ`, `ϑ`, `ϵ`, `ς`, the micro sign `µ`, the increment sign `∆` or a mathematical italic letter) SHALL be treated as the letter it is a form of, inside names as well as alone.

#### Scenario: Two forms of phi

- **WHEN** one cell runs `ϕ: 2$` and the next runs `φ + 1;`
- **THEN** the result SHALL be `3` (needs Maxima)

#### Scenario: Pasted italic letters

- **WHEN** a cell's input is `𝑥 + 𝛼;`
- **THEN** the input sent to Maxima SHALL be `x + alpha;`

### Requirement: Symbols with one meaning become Maxima's operators

Characters with one clear meaning in Maxima SHALL become it: operator forms (`−`, `×`, `·`, `⋅`, `∗`, `÷`, `∕`, `⁄`, `≤`, `≥`, `≠` and their variants), the logical operators `∧`, `∨` and `¬` (as `and`, `or` and `not`), `∞` (as `inf`), the constants `ⅈ`, `ⅇ` and `ℯ`, superscript runs (as powers) and subscript runs (as indices). `√` followed by a bracket, a number or a name SHALL become `sqrt`.

#### Scenario: Pasted minus and multiplication

- **WHEN** a cell's input is `2 × 3 − 1;`
- **THEN** the input sent to Maxima SHALL be `2 * 3 - 1;`

#### Scenario: Logic

- **WHEN** a cell's input is `a∧¬b;`
- **THEN** the input sent to Maxima SHALL be `a and not b;`

#### Scenario: Square roots

- **WHEN** a cell's input is `√2 + √(x+1) + √α;`
- **THEN** the input sent to Maxima SHALL be `sqrt(2) + sqrt(x+1) + sqrt(alpha);`

#### Scenario: Subscripts and superscripts

- **WHEN** a cell's input is `aₙ₊₁ = x²;`
- **THEN** the input sent to Maxima SHALL be `a[n+1] = x^(2);`

### Requirement: Unclear characters are rejected with a hint

A character in code with no clear meaning in Maxima SHALL stop the run before Maxima sees the input. The error SHALL name the character and its code point, the line when the input has more than one, and, where the table gives one, what to type instead. This SHALL include Greek capitals and omicron that look like Latin letters, styled letters other than italic, double-struck letters, `°`, `∑`, `∏`, `∫`, `∂`, `≈`, `±`, smart quotes, dashes, vulgar fractions, bidirectional control characters, `√` in any form not covered above, two subscript letters together, and a digit directly followed by a translated letter.

#### Scenario: Approximately equal

- **WHEN** a cell's input is `a ≈ b;`
- **THEN** the cell SHALL fail without running, with an error that contains `≈`, `U+2248` and `abs(a - b)`

#### Scenario: Lookalike capital

- **WHEN** a cell's input is `Α + 1;` with a Greek capital alpha
- **THEN** the cell SHALL fail without running, with an error that tells the user to type the Latin `A`

#### Scenario: Degrees

- **WHEN** a cell's input is `sin(30°);`
- **THEN** the cell SHALL fail without running, with an error that suggests `30*%pi/180`

#### Scenario: Implicit multiplication

- **WHEN** a cell's input is `2π;`
- **THEN** the cell SHALL fail without running, with an error that suggests `2*π`

#### Scenario: Expression over MCP

- **WHEN** an MCP client evaluates the expression `∑ k`
- **THEN** the tool SHALL return an error that suggests `sum(`

### Requirement: Pasted text is cleaned

No-break and other space characters SHALL become spaces. Zero-width characters and soft hyphens SHALL be removed. Full-width ASCII forms and ligatures SHALL become their ASCII characters. Code SHALL be normalised to NFC, so a letter with a combining accent is the same as the precomposed letter.

#### Scenario: No-break space

- **WHEN** a cell's input is `a` followed by U+00A0 and `+ b;`
- **THEN** the input sent to Maxima SHALL be `a + b;`

#### Scenario: Combining accent

- **WHEN** one cell runs `é: 1$` with a precomposed `é`, and the next runs `é + 1;` with `e` followed by U+0301
- **THEN** the result SHALL be `2` (needs Maxima)

### Requirement: Other letters pass through

A letter not covered above, such as an accented Latin letter or a Cyrillic or CJK letter, SHALL reach Maxima as typed.

#### Scenario: Accented name

- **WHEN** a cell's input is `größe: 2;`
- **THEN** the input sent to Maxima SHALL be `größe: 2;`

### Requirement: Translated names display as typed

In rendered output, a name produced from a Greek letter SHALL display as that letter, in the form typed: `θ`, `ε` and `φ` SHALL display as θ, ε and φ, not ϑ, ϵ and ϕ.

#### Scenario: Epsilon, phi and theta

- **WHEN** a cell runs `[ε, φ, θ];`
- **THEN** the LaTeX output SHALL contain `\varepsilon`, `\varphi` and `\theta`, and not `\vartheta` (needs Maxima)
