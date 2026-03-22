# Discoverability & User Guidance Plan

## Problem

Maxima has ~1500 functions but poor discoverability. Users don't know what functions exist, what arguments they take, or what to try next. Error messages are cryptic. There's no autocomplete, no inline docs, no contextual hints.

## Architecture Decision: Rust-First

All data, search, matching, and analysis logic lives in Rust, exposed via Tauri IPC commands. The TypeScript/React frontend is purely a rendering layer that calls commands and displays results.

Benefits:
- Single source of truth for function metadata, error patterns, and suggestion rules
- Catalog data can be used by both UI features and the parser (e.g. error enhancement happens during parsing)
- Rust's type system ensures catalog data integrity at compile time
- No data duplication between layers

```
React (rendering)  <--IPC-->  Rust (all logic)
  CommandPalette               catalog::search()
  AutocompletePopup            catalog::complete()
  CellSuggestions              suggestions::for_output()
  EnhancedError                parser::parse_output() (enhanced)
  TemplateChooser              notebooks::list_templates()
```

## Phases

```
Phase A: Function Catalog in Rust (Foundation)
   |
   +-- Phase B: Command Palette (Cmd+K)
   |
   +-- Phase C: Rich Error Messages (in parser)
   |
   +-- Phase E: Autocomplete
   |
Phase D: Contextual Suggestions (independent, benefits from A)
   |
Phase F: Starter Notebooks (best after C+D)
```

Recommended order: A -> D -> B -> C -> F -> E

---

## Phase A: Function Catalog in Rust

### New Rust module: `src-tauri/src/catalog/`

#### Types: `src-tauri/src/catalog/types.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaximaFunction {
    pub name: String,
    pub signatures: Vec<String>,
    pub description: String,
    pub category: FunctionCategory,
    pub examples: Vec<FunctionExample>,
    #[serde(default)]
    pub see_also: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionExample {
    pub input: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionCategory {
    Calculus,
    Algebra,
    LinearAlgebra,
    Simplification,
    Solving,
    Plotting,
    Trigonometry,
    NumberTheory,
    Polynomials,
    Series,
    Combinatorics,
    Programming,
    IO,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub function: MaximaFunction,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResult {
    pub name: String,
    pub signature: String,      // first signature for display
    pub description: String,
    pub insert_text: String,    // what to insert: "integrate()"
}
```

#### Data: `src-tauri/src/catalog/data.rs`

Embed the catalog as a static JSON string using `include_str!` and deserialize at startup, or define it as Rust data directly. ~150 functions organized by category:

| Category | ~Count | Key functions |
|----------|--------|---------------|
| Calculus | 20 | `integrate`, `diff`, `limit`, `taylor`, `sum`, `product` |
| Algebra/Simplification | 25 | `expand`, `factor`, `simplify`, `ratsimp`, `subst`, `ev` |
| Solving | 10 | `solve`, `algsys`, `linsolve`, `find_root`, `ode2` |
| Linear Algebra | 15 | `matrix`, `determinant`, `eigenvalues`, `invert`, `transpose` |
| Plotting | 5 | `plot2d`, `plot3d`, `draw2d` |
| Trigonometry | 10 | `sin`, `cos`, `tan`, `asin`, `trigreduce` |
| Number Theory | 10 | `gcd`, `lcm`, `primep`, `ifactors` |
| Polynomials | 10 | `coeff`, `degree`, `divide`, `gfactor` |
| Series | 5 | `taylor`, `powerseries` |
| Programming | 10 | `block`, `if`, `for`, `print`, `display` |
| Combinatorics | 5 | `binomial`, `factorial` |
| Other | 25 | `describe`, `example`, `apropos`, `kill`, `load`, `float` |

Use `include_str!("catalog.json")` with a `src-tauri/src/catalog/catalog.json` file for easy editing.

#### Search: `src-tauri/src/catalog/search.rs`

```rust
pub struct Catalog {
    functions: Vec<MaximaFunction>,
}

impl Catalog {
    pub fn load() -> Self;
    pub fn search(&self, query: &str) -> Vec<SearchResult>;
    pub fn complete(&self, prefix: &str) -> Vec<CompletionResult>;
    pub fn get(&self, name: &str) -> Option<&MaximaFunction>;
    pub fn by_category(&self, cat: &FunctionCategory) -> Vec<&MaximaFunction>;
    pub fn categories(&self) -> Vec<FunctionCategory>;
    pub fn find_similar(&self, name: &str, max_distance: usize) -> Vec<String>;
}
```

Search scoring: exact prefix > word-start match > substring > fuzzy (character-level). `find_similar` uses Levenshtein distance for "did you mean?" suggestions.

#### Add to AppState: `src-tauri/src/state.rs`

```rust
pub struct AppState {
    pub process: Arc<Mutex<Option<MaximaProcess>>>,
    pub status: Arc<Mutex<SessionStatus>>,
    pub catalog: Catalog,  // immutable, no mutex needed
}
```

#### Tauri commands: `src-tauri/src/commands/catalog.rs`

```rust
#[tauri::command]
pub fn search_functions(state: State<AppState>, query: String) -> Vec<SearchResult>;

#[tauri::command]
pub fn complete_function(state: State<AppState>, prefix: String) -> Vec<CompletionResult>;

#[tauri::command]
pub fn get_function(state: State<AppState>, name: String) -> Option<MaximaFunction>;

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> Vec<(FunctionCategory, Vec<MaximaFunction>)>;
```

#### TypeScript types: `src/types/catalog.ts`

Mirror the Rust types for the frontend to consume.

#### Client: `src/lib/catalog-client.ts`

```typescript
export async function searchFunctions(query: string): Promise<SearchResult[]>;
export async function completeFunction(prefix: string): Promise<CompletionResult[]>;
export async function getFunction(name: string): Promise<MaximaFunction | null>;
export async function listCategories(): Promise<[FunctionCategory, MaximaFunction[]][]>;
```

### Files

| Action | Path |
|--------|------|
| Create | `src-tauri/src/catalog/mod.rs` |
| Create | `src-tauri/src/catalog/types.rs` |
| Create | `src-tauri/src/catalog/data.rs` |
| Create | `src-tauri/src/catalog/search.rs` |
| Create | `src-tauri/src/catalog/catalog.json` |
| Create | `src-tauri/src/commands/catalog.rs` |
| Create | `src/types/catalog.ts` |
| Create | `src/lib/catalog-client.ts` |
| Modify | `src-tauri/src/state.rs` -- add `catalog` field |
| Modify | `src-tauri/src/lib.rs` -- register catalog commands |
| Modify | `src-tauri/src/commands/mod.rs` -- add `pub mod catalog` |

---

## Phase B: Command Palette (Cmd+K)

Modal searchable function browser. Calls `search_functions` and `list_categories` Tauri commands.

### Component: `src/components/CommandPalette.tsx`

- Full-screen overlay with centered modal
- Text input at top with autofocus
- On each keystroke, calls `searchFunctions(query)` (debounced ~100ms)
- Empty query: show all functions grouped by category (from `listCategories()`)
- Results show: function name (bold), first signature, description
- Keyboard: Arrow keys navigate, Enter inserts, Escape closes
- Selection inserts the function template (e.g. `integrate()`) into the active cell

### Store changes: `src/store/notebookStore.ts`

```typescript
activeCellId: string | null
setActiveCellId: (id: string | null) => void
insertTextInActiveCell: (text: string) => void
```

### Integration

- `Cell.tsx`: call `setActiveCellId(cell.id)` on textarea focus
- `App.tsx`: listen for Cmd+K / Ctrl+K, toggle command palette

### Files

| Action | Path |
|--------|------|
| Create | `src/components/CommandPalette.tsx` |
| Modify | `src/store/notebookStore.ts` -- add active cell tracking + text insertion |
| Modify | `src/components/Cell.tsx` -- set active cell on focus |
| Modify | `src/App.tsx` -- Cmd+K handler, render palette |
| Modify | `src/styles/global.css` -- palette styles |

---

## Phase C: Rich Error Messages

Move error enhancement into the Rust parser so `EvalResult` includes structured error info, not just a raw string.

### Extend EvalResult: `src-tauri/src/maxima/types.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub cell_id: String,
    pub text_output: String,
    pub latex: Option<String>,
    pub plot_svg: Option<String>,
    pub error: Option<String>,           // raw error (kept for fallback)
    pub error_info: Option<ErrorInfo>,   // NEW: structured error
    pub is_error: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub title: String,                   // "Division by Zero"
    pub explanation: String,             // friendly explanation
    pub suggestion: Option<String>,      // "Try checking your denominator"
    pub example: Option<String>,         // correct usage example
    pub did_you_mean: Vec<String>,       // similar function names from catalog
    pub correct_signatures: Vec<String>, // if wrong arg count, show correct sigs
}
```

### Error pattern matching: `src-tauri/src/maxima/errors.rs`

```rust
pub fn enhance_error(raw_error: &str, catalog: &Catalog) -> Option<ErrorInfo>;
```

15-20 patterns:

| Pattern | Title | Action |
|---------|-------|--------|
| `incorrect syntax: X is not a prefix operator` | Syntax Error | explain operator usage |
| `Premature termination` | Incomplete Expression | missing semicolon/paren |
| `Too few arguments supplied to X` | Wrong Argument Count | lookup `X` in catalog, show signatures |
| `Too many arguments supplied to X` | Wrong Argument Count | lookup `X` in catalog, show signatures |
| `undefined variable` | Undefined Variable | suggest `:` for assignment |
| `expt: undefined: 0 to a negative exponent` | Division by Zero | explain |
| `Maxima encountered a Lisp error` | Internal Error | suggest restart |
| `defint: integral is divergent` | Divergent Integral | explain convergence |
| `algsys: inconsistent equations` | No Solution | equations are contradictory |
| `rat: replaced` | Rational Approximation | info only, reduce severity |
| `Is X positive, negative, or zero?` | Missing Assumption | needs `assume()` |
| `all rows must be the same length` | Matrix Dimension Error | check row lengths |
| `is not a polynomial` | Not a Polynomial | suggest `find_root` |
| `loadfile: failed to load` | Package Not Found | package name help |

For argument-count errors: extract function name, look up in catalog, populate `correct_signatures`. For undefined names: call `catalog.find_similar()` for Levenshtein-based "did you mean?" suggestions.

### Integration into parser: `src-tauri/src/maxima/parser.rs`

`parse_output` needs access to the `Catalog` to call `enhance_error`. Change its signature:

```rust
pub fn parse_output(cell_id: &str, lines: &[String], duration_ms: u64, catalog: &Catalog) -> EvalResult;
```

After building the raw error string, call `enhance_error(raw_error, catalog)` to populate `error_info`.

### Update protocol: `src-tauri/src/maxima/protocol.rs`

Pass `catalog` through `evaluate()` to `parse_output()`.

### Frontend: `src/components/EnhancedErrorOutput.tsx`

Renders `ErrorInfo` when present:
- Title in bold
- Friendly explanation
- "Did you mean" suggestions as clickable links
- Correct signatures if applicable
- Collapsible raw error
- Falls back to raw error display when `error_info` is null

### Update TypeScript types: `src/types/maxima.ts`

Add `ErrorInfo` interface and `error_info` field to `EvalResult`.

### Files

| Action | Path |
|--------|------|
| Create | `src-tauri/src/maxima/errors.rs` |
| Create | `src/components/EnhancedErrorOutput.tsx` |
| Modify | `src-tauri/src/maxima/types.rs` -- add `ErrorInfo`, `error_info` field |
| Modify | `src-tauri/src/maxima/parser.rs` -- call `enhance_error`, accept catalog param |
| Modify | `src-tauri/src/maxima/protocol.rs` -- pass catalog to parser |
| Modify | `src-tauri/src/maxima/mod.rs` -- add `pub mod errors` |
| Modify | `src-tauri/src/commands/evaluate.rs` -- pass catalog to protocol |
| Modify | `src/types/maxima.ts` -- add `ErrorInfo` |
| Modify | `src/components/CellOutput.tsx` -- use `EnhancedErrorOutput` |
| Modify | `src/styles/global.css` -- error display styles |

---

## Phase D: Contextual Suggestions

Suggestion logic in Rust, exposed via a Tauri command. Frontend renders chips.

### Rust module: `src-tauri/src/suggestions/`

#### Types: `src-tauri/src/suggestions/types.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub label: String,        // "Simplify"
    pub template: String,     // "ratsimp(%)"
    pub description: String,  // tooltip text
    pub action: Option<String>,  // When set, triggers a frontend action (e.g. "save_svg")
}
```

#### Rules: `src-tauri/src/suggestions/rules.rs`

```rust
pub fn suggestions_for_output(input: &str, output: &EvalResult) -> Vec<Suggestion>;
```

10-15 rules using string/regex matching on input and output:

| Trigger | Suggestions |
|---------|-------------|
| Any expression output | `simplify(%)`, `factor(%)`, `expand(%)` |
| Matrix in input/output | `determinant(%)`, `eigenvalues(%)`, `invert(%)` |
| Equation (`=` in output) | `solve(%, x)`, `rhs(%)` |
| After `diff()` | `integrate(%, x)` |
| After `integrate()` | `diff(%, x)` |
| List output | `length(%)`, `sort(%)`, `first(%)` |
| Trig in output | `trigsimp(%)`, `trigexpand(%)` |
| Numeric result | `float(%)`, `bfloat(%)` |
| After `solve()` | `map(rhs, %)` |
| Plot output (`plot_svg` present) | "Save SVG" (action, not eval) |

Return max 5 suggestions, most relevant first. When `plot_svg` is present, only plot-specific action suggestions are returned (math suggestions are suppressed since residual text from Maxima's plot return value would trigger false matches like Length/Sort).

#### Tauri command: `src-tauri/src/commands/suggestions.rs`

```rust
#[tauri::command]
pub fn get_suggestions(input: String, output: EvalResult) -> Vec<Suggestion>;
```

### Frontend: `src/components/CellSuggestions.tsx`

- On successful cell output, calls `getSuggestions(input, output)`
- Renders chips in a row below the cell output
- Clicking a chip creates a new cell with the template pre-filled

### Store change: `src/store/notebookStore.ts`

Add `addCellWithInput(afterId: string, input: string)` action.

### Files

| Action | Path |
|--------|------|
| Create | `src-tauri/src/suggestions/mod.rs` |
| Create | `src-tauri/src/suggestions/types.rs` |
| Create | `src-tauri/src/suggestions/rules.rs` |
| Create | `src-tauri/src/commands/suggestions.rs` |
| Create | `src/components/CellSuggestions.tsx` |
| Create | `src/lib/suggestions-client.ts` |
| Create | `src/types/suggestions.ts` |
| Modify | `src-tauri/src/lib.rs` -- register `get_suggestions` command |
| Modify | `src-tauri/src/commands/mod.rs` -- add `pub mod suggestions` |
| Modify | `src/components/Cell.tsx` -- render `CellSuggestions` below output |
| Modify | `src/store/notebookStore.ts` -- add `addCellWithInput` |
| Modify | `src/styles/global.css` -- chip styles |

---

## Phase E: Autocomplete

Calls the `complete_function` Tauri command from Phase A. Frontend handles popup positioning and keyboard interaction.

### Design decision

If CodeMirror (existing Phase 2) is coming soon, build autocomplete as a CodeMirror extension using the `completeFunction` client. CodeMirror's `@codemirror/autocomplete` handles all the hard parts (positioning, keyboard, rendering).

If staying with textarea, build a lightweight overlay:

### Textarea approach (if needed)

**Hook**: `src/hooks/useAutocomplete.ts`
- Extract word at cursor via `selectionStart`
- If >= 2 chars, call `completeFunction(prefix)` (debounced)
- Handle Tab/Enter (accept), Escape (dismiss), Arrow keys (navigate)

**Popup**: `src/components/AutocompletePopup.tsx`
- Positioned using mirror-div technique for cursor coordinates
- Shows name, signature, description for each completion
- Max 8 visible results

**Integration**: `Cell.tsx` -- autocomplete keys intercepted before Shift+Enter.

### Files (textarea version)

| Action | Path |
|--------|------|
| Create | `src/hooks/useAutocomplete.ts` |
| Create | `src/components/AutocompletePopup.tsx` |
| Create | `src/lib/textarea-caret.ts` |
| Modify | `src/components/Cell.tsx` |
| Modify | `src/styles/global.css` |

---

## Phase F: Starter Notebooks

Notebook content and template listing in Rust. Frontend renders the chooser.

### Rust module: `src-tauri/src/notebooks/`

#### Types: `src-tauri/src/notebooks/types.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookTemplate {
    pub id: String,
    pub title: String,
    pub description: String,
    pub cells: Vec<NotebookCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCell {
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub cell_count: usize,
}
```

#### Data: `src-tauri/src/notebooks/data.rs`

Embed notebook templates using `include_str!` from JSON files:

| File | Topic |
|------|-------|
| `notebooks/welcome.json` | Getting Started -- arithmetic, variables, basic algebra |
| `notebooks/calculus.json` | Derivatives, integrals, limits, Taylor series |
| `notebooks/linear-algebra.json` | Matrices, determinants, eigenvalues |
| `notebooks/equations.json` | solve(), algsys(), find_root(), ode2() |
| `notebooks/programming.json` | Functions, loops, conditionals |
| `notebooks/plotting.json` | 2D, 3D, parametric plots |

#### Tauri commands: `src-tauri/src/commands/notebooks.rs`

```rust
#[tauri::command]
pub fn list_templates() -> Vec<TemplateSummary>;

#[tauri::command]
pub fn get_template(id: String) -> Option<NotebookTemplate>;

#[tauri::command]
pub fn get_has_seen_welcome(app: AppHandle) -> Result<bool, AppError>;

#[tauri::command]
pub fn set_has_seen_welcome(app: AppHandle) -> Result<(), AppError>;
```

### Frontend: `src/components/TemplateChooser.tsx`

Modal showing templates from `listTemplates()`. On selection, loads cells into store.

### Store: `src/store/notebookStore.ts`

Add `loadNotebook(cells: { input: string }[])` action.

### Files

| Action | Path |
|--------|------|
| Create | `src-tauri/src/notebooks/mod.rs` |
| Create | `src-tauri/src/notebooks/types.rs` |
| Create | `src-tauri/src/notebooks/data.rs` |
| Create | `src-tauri/src/notebooks/welcome.json` |
| Create | `src-tauri/src/notebooks/calculus.json` |
| Create | `src-tauri/src/notebooks/linear-algebra.json` |
| Create | `src-tauri/src/notebooks/equations.json` |
| Create | `src-tauri/src/notebooks/programming.json` |
| Create | `src-tauri/src/notebooks/io.rs` |
| Create | `src-tauri/src/commands/notebooks.rs` |
| Create | `src/components/TemplateChooser.tsx` |
| Create | `src/lib/notebooks-client.ts` |
| Create | `src/types/notebooks.ts` |
| Modify | `src-tauri/src/commands/config.rs` -- add `has_seen_welcome` to AppConfig |
| Modify | `src-tauri/src/lib.rs` -- register notebook commands |
| Modify | `src-tauri/src/commands/mod.rs` -- add `pub mod notebooks` |
| Modify | `src/components/Toolbar.tsx` -- Templates button |
| Modify | `src/App.tsx` -- first-run detection, template chooser |
| Modify | `src/store/notebookStore.ts` -- add `loadNotebook` |
| Modify | `src/styles/global.css` -- chooser styles |

---

## Full File Summary

### New Rust files

| File | Phase |
|------|-------|
| `src-tauri/src/catalog/mod.rs` | A |
| `src-tauri/src/catalog/types.rs` | A |
| `src-tauri/src/catalog/data.rs` | A |
| `src-tauri/src/catalog/search.rs` | A |
| `src-tauri/src/catalog/catalog.json` | A |
| `src-tauri/src/commands/catalog.rs` | A |
| `src-tauri/src/maxima/errors.rs` | C |
| `src-tauri/src/suggestions/mod.rs` | D |
| `src-tauri/src/suggestions/types.rs` | D |
| `src-tauri/src/suggestions/rules.rs` | D |
| `src-tauri/src/commands/suggestions.rs` | D |
| `src-tauri/src/notebooks/mod.rs` | F |
| `src-tauri/src/notebooks/types.rs` | F |
| `src-tauri/src/notebooks/data.rs` | F |
| `src-tauri/src/notebooks/*.json` | F |
| `src-tauri/src/commands/notebooks.rs` | F |

### New TypeScript files

| File | Phase |
|------|-------|
| `src/types/catalog.ts` | A |
| `src/lib/catalog-client.ts` | A |
| `src/components/CommandPalette.tsx` | B |
| `src/components/EnhancedErrorOutput.tsx` | C |
| `src/types/suggestions.ts` | D |
| `src/lib/suggestions-client.ts` | D |
| `src/components/CellSuggestions.tsx` | D |
| `src/hooks/useAutocomplete.ts` | E |
| `src/components/AutocompletePopup.tsx` | E |
| `src/lib/textarea-caret.ts` | E |
| `src/types/notebooks.ts` | F |
| `src/lib/notebooks-client.ts` | F |
| `src/components/TemplateChooser.tsx` | F |

### Modified files

| File | Phases |
|------|--------|
| `src-tauri/src/lib.rs` | A, D, F |
| `src-tauri/src/commands/mod.rs` | A, D, F |
| `src-tauri/src/state.rs` | A |
| `src-tauri/src/maxima/types.rs` | C |
| `src-tauri/src/maxima/parser.rs` | C |
| `src-tauri/src/maxima/protocol.rs` | C |
| `src-tauri/src/maxima/mod.rs` | C |
| `src-tauri/src/commands/evaluate.rs` | C |
| `src-tauri/src/commands/config.rs` | F |
| `src/types/maxima.ts` | C |
| `src/store/notebookStore.ts` | B, D, F |
| `src/components/Cell.tsx` | B, D, E |
| `src/components/CellOutput.tsx` | C |
| `src/components/Toolbar.tsx` | F |
| `src/App.tsx` | B, F |
| `src/styles/global.css` | B, C, D, E, F |

## Verification

| Phase | Test |
|-------|------|
| A | `cargo test` on catalog search. Call `search_functions("integ")` from frontend, verify results. |
| B | Cmd+K opens palette, search filters, selection inserts into active cell. |
| C | Trigger `1/0`, `intgrate(x,x)`, `solve(x,x,x,x)` -- verify enhanced error with title, explanation, "did you mean", correct signatures. Run existing parser tests + new error enhancement tests. |
| D | Run `diff(x^2, x)`, verify suggestion chips appear, clicking inserts template in new cell. |
| E | Type `int` in cell, verify popup with `integrate`, Tab inserts. |
| F | First launch shows welcome notebook. Templates button opens chooser. |
