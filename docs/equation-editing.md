# Equation Editing in Code Cells

## Status: Design Discussion

Currently, code cells use a CodeMirror text editor with Maxima ASCII syntax. Users type expressions like `integrate(sin(x), x, 0, %pi)` and see rendered LaTeX output below. Unicode symbol input is supported via backslash completions (`\alpha` → `α`, `\_0` → `₀`, `\^2` → `²`), and the backend translates these to Maxima syntax automatically.

This document explores options for richer equation editing — moving toward visual math input rather than plain text.

## Options

### Option 1: Enhanced current approach (incremental)

Extend the existing backslash completion system with structural templates:

- `\frac` → template `( )/( )`
- `\int` → `integrate( )`
- `\sum` → `sum( )`
- `\diff` → `diff( )`

**Pros:**
- Minimal complexity
- Maxima stays the source of truth
- Full power of text editing (loops, functions, assignments all work)
- Builds on existing infrastructure

**Cons:**
- Still fundamentally ASCII text input
- No visual feedback until evaluation

### Option 2: Inline CodeMirror decorations

Keep CodeMirror with Maxima text as the underlying document, but render math expressions inline as widgets/decorations. For example, `integrate(sin(x), x, 0, %pi)` would display as a rendered integral symbol with proper limits while editing, but the actual buffer text remains Maxima.

Similar to how Mathematica renders input cells, or how VS Code shows inline type hints.

**Pros:**
- No conversion layer needed — Maxima is the source of truth
- Supports all Maxima syntax (loops, functions, assignments, blocks)
- User sees rendered math while editing
- Graceful degradation: if rendering fails, the text is still there

**Cons:**
- Complex CodeMirror plugin development
- Tricky cursor/selection behaviour around decoration widgets
- Need a Maxima→LaTeX renderer that works on partial/incomplete expressions
- Deciding what to render vs leave as text (e.g. `for` loops shouldn't be rendered)

### Option 3: MathLive as an alternative cell input mode

[MathLive](https://cortexjs.io/mathlive/) is a mature web component for visual math editing. Offer a per-cell toggle: code mode (current CodeMirror) vs equation mode (MathLive). MathLive produces LaTeX which gets translated to Maxima for evaluation.

**Pros:**
- Polished visual equation editing out of the box
- Great UX for users who think in math notation
- Active open-source project with good documentation

**Cons:**
- Requires a LaTeX→Maxima translator (see "The Hard Problem" below)
- Cannot handle Maxima-specific constructs (`:=`, `for` loops, `block`, `assume`)
- Likely limited to single-expression cells
- Two representations to keep in sync

### Option 4: MathQuill

[MathQuill](http://mathquill.com/) is an older visual math editor, used by Desmos. Same trade-offs as MathLive — produces LaTeX, needs translation to Maxima. More battle-tested but less actively maintained.

### Option 5: Hybrid — MathLive widgets inside CodeMirror

Rather than replacing the whole editor, embed MathLive widgets inside CodeMirror for specific expression fragments. User types `integrate(` and gets a visual widget for the integrand and bounds, while surrounding code (assignments, loops) stays as text.

**Pros:**
- Best of both worlds for mixed code/math cells
- Maxima-specific constructs remain as text

**Cons:**
- Very complex integration between two editor frameworks
- Bidirectional sync at sub-expression level
- Novel UI pattern that may confuse users

## The Hard Problem: LaTeX→Maxima Translation

All visual editing approaches (Options 3–5) require translating LaTeX back to Maxima. This is fundamentally difficult because LaTeX is a typesetting language, not an algebraic one. The same visual expression can map to different Maxima code depending on context.

Examples:

| LaTeX | Maxima | Difficulty |
|-------|--------|------------|
| `\frac{a}{b}` | `a/b` | Straightforward |
| `\frac{d}{dx} \sin(x)` | `diff(sin(x), x)` | Structural rewrite — `d/dx` is an operator, not a fraction |
| `\frac{\partial^2 f}{\partial x \partial y}` | `diff(f, x, 1, y, 1)` | Complex operator parsing |
| `\sum_{n=0}^{\infty} a_n x^n` | `sum(a[n] * x^n, n, 0, inf)` | Limits become function arguments |
| `x^2 y` | `x^2*y` | Implicit multiplication |
| `f'(x)` | `diff(f(x), x)` | Prime notation convention |
| `\int_0^\pi \sin(x)\,dx` | `integrate(sin(x), x, 0, %pi)` | Extract variable of integration from `dx` |
| `\lim_{x \to 0}` | `limit(..., x, 0)` | Subscript is structured argument |

A partial solution could use MathLive's built-in Compute Engine, which can convert LaTeX to a structured math AST. From there, generating Maxima would be more tractable than parsing raw LaTeX strings.

## Recommendation

**Phase 1 (current):** Continue enhancing the backslash completion system. Add structural templates for common operations.

**Phase 2:** Implement Option 2 (inline CodeMirror decorations). This gives visual math rendering without requiring a translation layer. The Maxima text stays as the source of truth, and decorations are purely presentational. Start with rendering simple sub-expressions (fractions, superscripts, Greek letters) and expand coverage over time.

**Phase 3 (optional):** Explore Option 3 (MathLive toggle) for pure-expression cells. Investigate MathLive's Compute Engine as a bridge for LaTeX→Maxima translation. This would only apply to cells containing a single mathematical expression, not general Maxima code.
