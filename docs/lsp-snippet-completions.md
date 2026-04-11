# LSP Snippet Completions

## Problem

When a user selects `ax_plot2d` from the completion list, the LSP inserts `ax_plot2d()` with no argument guidance. AI-powered completions (e.g. Copilot) then fill in arguments speculatively, sometimes incorrectly (e.g. suggesting `[y,...]` ranges for `ax_plot2d` which only accepts `[x,...]`-style explicit ranges).

The LSP already provides **signature help** (parameter hints shown while typing inside parens), but this only appears after the user has already started typing arguments. It doesn't prevent incorrect AI suggestions from appearing first.

## Proposed Solution

Use LSP **snippet insert text** on `CompletionItem`s to pre-fill argument structure with tab stops. When the user selects a function from the completion list, they get a structured template they can tab through, which also suppresses AI-generated argument suggestions.

Example for `ax_plot2d`:

```
ax_plot2d(${1:expr}, [${2:x}, ${3:-5}, ${4:5}])$0
```

This inserts `ax_plot2d(expr, [x, -5, 5])` with the cursor on `expr`, and Tab advances through `x`, `-5`, `5`.

### LSP mechanics

Set two fields on the `CompletionItem`:

- `insert_text`: the snippet string with `$1`, `${1:default}` placeholders
- `insert_text_format`: `InsertTextFormat::SNIPPET` (value `2`)

## Scope

### Tier 1: Custom snippets (ax_plotting functions)

Functions with specific argument patterns and useful defaults. Store a `snippet` field per signature in `ax_draw_context.json`:

| Function | Snippet |
|----------|---------|
| `ax_plot2d` | `ax_plot2d(${1:expr}, [${2:x}, ${3:-5}, ${4:5}])$0` |
| `ax_polar` | `ax_polar(${1:expr}, [${2:\u03b8}, ${3:0}, ${4:2*%pi}])$0` |
| `ax_draw2d` | `ax_draw2d(${1:object})$0` |
| `ax_draw3d` | `ax_draw3d(${1:object})$0` |
| `ax_contour` | `ax_contour(${1:expr}, ${2:x}, ${3:-3}, ${4:3}, ${5:y}, ${6:-3}, ${7:3})$0` |
| `ax_heatmap` | `ax_heatmap(${1:expr}, ${2:x}, ${3:-3}, ${4:3}, ${5:y}, ${6:-3}, ${7:3})$0` |
| `ax_bar` | `ax_bar(${1:categories}, ${2:values})$0` |
| `ax_histogram` | `ax_histogram(${1:data})$0` |
| `ax_vector_field` | `ax_vector_field(${1:Fx}, ${2:Fy}, ${3:x}, ${4:-3}, ${5:3}, ${6:y}, ${7:-3}, ${8:3})$0` |
| `ax_streamline` | `ax_streamline(${1:Fx}, ${2:Fy}, ${3:x}, ${4:-3}, ${5:3}, ${6:y}, ${7:-3}, ${8:3})$0` |

### Tier 2: Auto-generated snippets (core Maxima functions)

For functions without custom snippets, auto-generate from the existing signature strings. Parse `"integrate(f, x, a, b)"` and produce `integrate(${1:f}, ${2:x}, ${3:a}, ${4:b})$0`.

This covers high-value functions like:

- `integrate(expr, x, a, b)` / `integrate(expr, x)`
- `diff(expr, x, n)` / `diff(expr, x)`
- `limit(expr, x, val)` / `limit(expr, x, val, dir)`
- `plot2d(expr, [x, a, b])`
- `taylor(expr, x, a, n)`
- `solve(expr, x)` / `solve([eq1, eq2], [x, y])`
- `sum(expr, i, lo, hi)` / `product(expr, i, lo, hi)`

The auto-generation logic:

1. Take the first signature string (e.g. `"integrate(f, x, a, b)"`)
2. Extract the parameter list between parens
3. Split on `,` (respecting bracket nesting for `[var, min, max]` groups)
4. Assign sequential tab stop numbers
5. Use parameter names as placeholder defaults

Simple functions like `sin(x)`, `expand(expr)` also get snippets this way, which is harmless -- a single `${1:x}` placeholder is no worse than bare parens and still provides the parameter name as a hint.

### Tier 3: Skip

Functions with complex variadic signatures (e.g. `printf`, `apply`) or where the signature is `"f([args])"` are better left as `f()` with signature help only.

## Implementation

### Data layer (aximar-core)

Add an optional `snippet` field to the function representation in `ax_draw_context.json`:

```json
{
  "name": "ax_plot2d",
  "signatures": ["ax_plot2d(expr, [var, min, max], options)"],
  "snippet": "ax_plot2d(${1:expr}, [${2:x}, ${3:-5}, ${4:5}])$0",
  ...
}
```

In `catalog/search.rs`, extend `CompletionResult` with an optional `snippet` field. If present, use it; otherwise auto-generate from the signature.

### Auto-generation (aximar-core)

Add a function `generate_snippet(signature: &str) -> Option<String>` that:

1. Finds the function name and param list from the signature
2. Splits params respecting `[...]` bracket groups
3. Numbers tab stops sequentially
4. Returns `None` for empty param lists (no snippet needed)

### LSP layer (maxima-lsp)

In `completion.rs`, when building `CompletionItem`:

```rust
let (insert_text, insert_text_format) = if let Some(snippet) = &cr.snippet {
    (snippet.clone(), Some(InsertTextFormat::SNIPPET))
} else {
    (format!("{}()", cr.name), Some(InsertTextFormat::PLAIN_TEXT))
};
```

Same pattern for package function completions and document symbol completions (auto-generate for user-defined functions from their parsed parameter lists).

### Package function completions

Apply auto-generation to package functions too. `PackageCatalog::complete_functions` already returns signatures -- generate snippets from those in the same way.

## Testing

- Unit test `generate_snippet` with various signature patterns
- Unit test that custom snippets from `ax_draw_context.json` are returned for ax_plotting functions
- Unit test that auto-generated snippets have correct tab stop numbering
- Integration test: verify `CompletionItem`s have `insert_text_format: SNIPPET` when snippets are present
