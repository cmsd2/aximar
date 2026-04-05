# Interactive Plotting with Plotly.js

Aximar provides interactive plotting functions that render with [Plotly.js](https://plotly.com/javascript/) instead of gnuplot. These produce pannable, zoomable, hover-enabled charts directly in the notebook.

## Functions

### `ax_plot2d` — Simple 2D Plotting

Quick function plotting with automatic `explicit()` wrapping.

```maxima
/* Single expression */
ax_plot2d(sin(x), [x, -π, π]);

/* Multiple expressions */
ax_plot2d([sin(x), cos(x)], [x, -5, 5]);

/* With options */
ax_plot2d(x^2, [x, -3, 3], title="Parabola", color="red");
```

**Syntax:** `ax_plot2d(expr_or_list, [var, min, max], options...)`

- First argument: a single expression or a list of expressions
- Second argument: the variable range `[var, min, max]`
- Remaining arguments: style and layout options (see below)

### `ax_draw2d` — 2D Drawing with Draw Objects

Uses Maxima's draw package object syntax for full control over trace types.

```maxima
/* Explicit curves with styling */
ax_draw2d(
  color="red", explicit(x^2, x, -3, 3),
  color="blue", explicit(x^3, x, -2, 2)
);

/* Parametric curve */
ax_draw2d(
  parametric(cos(t), sin(t), t, 0, 2*π),
  line_width=3,
  title="Unit Circle"
);

/* Scatter points */
ax_draw2d(
  points([[1,1],[2,4],[3,9],[4,16]]),
  marker_symbol="diamond",
  marker_size=10
);

/* Implicit equation (rendered as contour at zero) */
ax_draw2d(implicit(x^2 + y^2 = 4, x, -3, 3, y, -3, 3));

/* Mixed types */
ax_draw2d(
  explicit(sin(x), x, -π, π),
  points([[0,0],[π/2,1],[-π/2,-1]]),
  marker_size=8,
  color="red"
);
```

**Supported draw objects:**

| Object | Syntax | Description |
|--------|--------|-------------|
| `explicit` | `explicit(expr, var, lo, hi)` | A curve y=f(x) |
| `parametric` | `parametric(x(t), y(t), t, tlo, thi)` | A parametric curve |
| `points` | `points([[x1,y1],[x2,y2],...])` | Scatter points |
| `implicit` | `implicit(eqn, x, xlo, xhi, y, ylo, yhi)` | An implicit curve f(x,y)=0 |

### `ax_draw3d` — 3D Drawing

```maxima
/* 3D surface */
ax_draw3d(explicit(sin(x)*cos(y), x, -π, π, y, -π, π));

/* 3D scatter points */
ax_draw3d(
  points([[1,1,1],[2,2,4],[3,3,9]]),
  marker_size=5
);

/* With options */
ax_draw3d(
  explicit(x^2 - y^2, x, -2, 2, y, -2, 2),
  title="Saddle Surface",
  colorscale="Viridis"
);
```

**Supported 3D draw objects:**

| Object | Syntax | Description |
|--------|--------|-------------|
| `explicit` | `explicit(expr, x, xlo, xhi, y, ylo, yhi)` | A surface z=f(x,y) |
| `points` | `points([[x1,y1,z1],[x2,y2,z2],...])` | 3D scatter points |

## Style Options

Style options apply to subsequent draw objects until overridden. They use **Plotly-native naming**.

| Option | Default | Description | Plotly mapping |
|--------|---------|-------------|----------------|
| `color` | auto | Line/marker color | `line.color` / `marker.color` |
| `fill_color` | none | Fill area color | `fillcolor` |
| `opacity` | 1.0 | Trace opacity (0–1) | `opacity` |
| `line_width` | 2 | Line width in pixels | `line.width` |
| `dash` | `"solid"` | Line dash style | `line.dash` |
| `marker_symbol` | `"circle"` | Marker shape | `marker.symbol` |
| `marker_size` | 6 | Marker size in pixels | `marker.size` |
| `name` | auto | Legend entry name | `name` |
| `fill` | none | Fill mode | `fill` |
| `colorscale` | none | Colorscale for 3D surfaces | `colorscale` |
| `nticks` | 500 (2D) / 50 (3D) | Sampling resolution | (internal) |

**Dash styles:** `"solid"`, `"dot"`, `"dash"`, `"dashdot"`, `"longdash"`, `"longdashdot"`

**Marker symbols:** `"circle"`, `"square"`, `"diamond"`, `"cross"`, `"x"`, `"triangle-up"`, `"triangle-down"`, `"star"`, and [many more](https://plotly.com/javascript/reference/scatter/#scatter-marker-symbol)

**Fill modes:** `"tozeroy"`, `"tozerox"`, `"tonexty"`, `"tonextx"`, `"toself"`

**Color values:** Maxima atoms (`red`, `blue`, `green`, etc.) or CSS strings (`"#ff0000"`, `"rgb(255,0,0)"`, `"steelblue"`)

### Style Accumulation

Options apply to all subsequent objects until changed:

```maxima
ax_draw2d(
  color="red", line_width=3,
  explicit(sin(x), x, -π, π),       /* red, width 3 */
  color="blue",
  explicit(cos(x), x, -π, π),       /* blue, width 3 (inherited) */
  line_width=1, dash="dot",
  explicit(sin(2*x), x, -π, π)      /* blue, width 1, dotted */
);
```

## Layout Options

Layout options are global (not per-trace) and control the plot frame.

| Option | Description | Example |
|--------|-------------|---------|
| `title` | Plot title | `title="My Plot"` |
| `xrange` | X-axis range | `xrange=[-5,5]` |
| `yrange` | Y-axis range | `yrange=[0,10]` |
| `zrange` | Z-axis range (3D) | `zrange=[-1,1]` |
| `xlabel` | X-axis label | `xlabel="Time (s)"` |
| `ylabel` | Y-axis label | `ylabel="Amplitude"` |
| `zlabel` | Z-axis label (3D) | `zlabel="Height"` |
| `grid` | Show grid lines | `grid=true` |
| `xaxis` | Show x-axis | `xaxis=false` |
| `yaxis` | Show y-axis | `yaxis=false` |
| `showlegend` | Show legend | `showlegend=true` |
| `aspect_ratio` | Lock aspect ratio | `aspect_ratio=true` |

## Interactivity

Plotly charts are fully interactive:

- **Pan/zoom** with mouse drag and scroll
- **Hover** to see data values
- **Mode bar** appears on hover with tools for zoom, pan, reset, and download
- **3D rotation** by click-and-drag on 3D plots
- **Legend** click to toggle trace visibility

## Comparison with `plot2d` / `draw2d`

| Feature | `plot2d`/`draw2d` (gnuplot) | `ax_plot2d`/`ax_draw2d` (Plotly) |
|---------|---------------------------|----------------------------------|
| Output | Static SVG | Interactive HTML/JS |
| Pan/zoom | No | Yes |
| Hover data | No | Yes |
| 3D rotation | No | Yes |
| Legend toggle | No | Yes |
| Export | SVG only | PNG, SVG, WebGL |
| Option naming | draw-style (`point_type`, `line_type`) | Plotly-native (`marker_symbol`, `dash`) |

## Architecture

1. The `ax_` functions sample expressions in Maxima and build a Plotly JSON spec
2. The JSON is written to a temp file (`.plotly.json`) in `maxima_tempdir`
3. The file path is printed to stdout
4. The Aximar parser detects the path, reads the file, validates the JSON, and populates `plot_data` on the cell output
5. The React frontend renders the spec using Plotly.js

This filesystem-based data transfer mirrors how gnuplot SVG plots already work, ensuring consistency and avoiding stdout size limits.
