# wxMaxima Feature Gap Analysis

An analysis of wxMaxima-specific functions and features that go beyond standard Maxima, and how they relate to Aximar.

## Background

wxMaxima extends Maxima with its own functions, variables, animation system, document model, and export capabilities. This document catalogues those extensions and identifies which ones represent gaps in Aximar's functionality.

## Already Covered by Aximar

### Inline Plotting (wx-prefixed wrappers)

wxMaxima provides ~15 `wx`-prefixed plotting functions (`wxplot2d`, `wxplot3d`, `wxdraw`, `wxdraw2d`, `wxdraw3d`, `wximplicit_plot`, `wxcontour_plot`, `wxhistogram`, `wxscatterplot`, `wxbarsplot`, `wxpiechart`, `wxboxplot`, `wxqdraw`). These behave identically to their standard Maxima equivalents but render output inline rather than opening an external gnuplot window.

**Aximar status:** Not a gap. Aximar achieves inline plot rendering by setting `run_viewer` to `false` and redirecting gnuplot output to SVG files, which are then displayed in the cell output. The result is the same — plots appear inline.

### Notebook / Document Model

wxMaxima provides a structured document with code cells, text cells, headings, and collapsible sections.

**Aximar status:** Covered. Aximar has its own cell-based notebook model with code and markdown cells, persisted in `.macnb` format.

### Typeset Mathematical Output

wxMaxima renders output using proper mathematical notation (fractions, roots, integrals).

**Aximar status:** Covered. Aximar uses `tex()` to get LaTeX from Maxima and renders it via KaTeX.

## Feature Gaps

### Major

#### Animation / Slider Controls

wxMaxima provides interactive animations driven by a parameter slider:

- `with_slider_draw(var, values, ...)` — 2D animation using `draw` package
- `with_slider_draw3d(var, values, ...)` — 3D animation with rotation
- `with_slider(var, values, ...)` — animation using `plot` commands
- `wxanimate()` — general animation function
- `wxanimate_draw()` / `wxanimate_draw3d()` — animation via `draw` package

Related variables:
- `wxanimate_framerate` — frames per second
- `wxanimate_autoplay` — auto-play on creation

**Why it matters:** Animations are valuable for teaching and interactive exploration of how curves, surfaces, or solutions change as a parameter varies. This is a genuinely interactive feature with no equivalent in standard Maxima.

**Possible implementation:** Could be implemented as a slider UI component in the cell output area that re-evaluates an expression with substituted parameter values, rendering each frame as an inline SVG.

#### `table_form()`

Renders a 2D list as a formatted HTML-like table rather than raw Maxima list output.

**Why it matters:** Displaying matrices and tabular data as actual tables is significantly more readable than nested list syntax. Relatively high impact for low implementation effort.

**Possible implementation:** Detect `table_form()` calls (or provide a custom Maxima function) and render the result as an HTML table in the cell output.

#### Notebook Export (HTML / LaTeX)

wxMaxima can export entire workbooks as HTML or LaTeX documents. Individual cells can be copied as LaTeX or MathML.

**Why it matters:** Important for sharing work with others who don't have Aximar installed, and for incorporating results into papers or reports.

**Status:** LaTeX export implemented via File > Export as LaTeX (Cmd+Shift+E). Supports multi-cell selection (shift-click range), toggles for code input and plot inclusion, and exports plots as images alongside the `.tex` file. HTML export is not yet implemented.

### Minor

#### `wxstatusbar(message)`

Displays a progress message in the status bar during long-running computations. The message updates with each call and clears when the command finishes.

**Why it matters:** Provides feedback during long computations. Currently Aximar shows a spinner but no progress detail from the computation itself.

#### `show_image(filename)`

Embeds an external image file directly into the worksheet.

**Why it matters:** Useful for documentation-style notebooks that mix computation with diagrams or reference images. Low priority since markdown cells could potentially link to images.

#### `wxdeclare_subscript()` and `wxsubscripts`

Controls whether variable names like `x_1` are displayed with subscript formatting (x₁).

**Why it matters:** Cosmetic improvement for mathematical readability. KaTeX already handles subscripts in LaTeX output, so the gap is primarily about controlling which variables get subscript treatment in non-LaTeX contexts.

#### `.wxm` / `.wxmx` File Import

- `.wxm` — plain-text worksheet format using Maxima comment syntax
- `.wxmx` — compressed XML archive with outputs and images

**Why it matters:** Would ease migration for users coming from wxMaxima. The `.wxm` format is relatively simple to parse. The `.wxmx` format is a zip archive containing XML and embedded images.

#### Plot Configuration Variables

- `wxplot_size` — resolution of embedded plots in pixels
- `wxplot_pngcairo` — use higher-quality pngcairo gnuplot terminal

**Why it matters:** Aximar already controls plot output format (SVG) and could expose size configuration through its own settings. Low priority since SVG is resolution-independent.

## Priority Summary

| Feature | Impact | Effort | Priority |
|---------|--------|--------|----------|
| Animation / sliders | High | High | 1 |
| `table_form()` | Medium | Low | 2 |
| Notebook export (HTML/LaTeX) | Medium | Medium | 3 | LaTeX done |
| `wxstatusbar()` progress | Low-Medium | Low | 4 |
| `.wxm` import | Low-Medium | Medium | 5 |
| `show_image()` | Low | Low | 6 |
| Subscript control | Low | Low | 7 |
| `.wxmx` import | Low | High | 8 |
| Plot size config | Low | Low | 9 |

## Conclusion

The most significant gap is **animation/slider support**, which is a genuinely interactive feature that standard Maxima cannot provide. The second most impactful addition would be **`table_form()`** for its readability benefits at low implementation cost. **Notebook export** rounds out the top three as an important workflow feature for sharing results.

The remaining wxMaxima extensions are either cosmetic, low-usage, or already effectively handled by Aximar's existing architecture (particularly the inline plotting, which Aximar solves via SVG redirection rather than wx-prefixed function wrappers).
