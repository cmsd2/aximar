use std::sync::OnceLock;

/// Embedded Lisp helpers for the plotting functions (loaded first).
///
/// Defines `ax__mktemp` plus the optional ndarray-support helpers
/// (`$ax__ndarray_p`, `$ax__ndarray_to_list`, `$ax__ndarray_to_matrix`).
/// Without the ndarray helpers, `ax__maybe_matrix` in `ax_plotting.mac`
/// returns an unsimplified `if` form when matrix-like input flows
/// through ax_heatmap / ax_bar / etc., which then crashes
/// `ax__float_matrix_to_json`.
const AX_PLOTTING_LISP: &str = include_str!("ax_plotting.lisp");

/// Embedded Maxima code defining ax_plot2d, ax_draw2d, ax_draw3d plotting functions.
///
/// These functions produce Plotly.js JSON specs written to temp files, which the
/// parser detects and reads (same pattern as gnuplot SVG files).
const AX_PLOTTING_MAC: &str = include_str!("ax_plotting.mac");

/// Returns the raw Lisp helper source (for integration tests that write to a file).
pub fn plotting_lisp_code() -> &'static str {
    AX_PLOTTING_LISP
}

/// Returns the Lisp helper bundle as a single `:lisp (progn …)` line
/// suitable for sending through Maxima stdin during session init.
///
/// Built once from `AX_PLOTTING_LISP` by stripping `;` line comments
/// and collapsing onto one physical line, since Maxima's `:lisp`
/// directive reads one form per line.  The progn wrapper keeps every
/// top-level form (defvars, defuns, in-package) inside a single form
/// so we can ship them in one go without the per-form acknowledgement
/// dance.
pub fn plotting_lisp_stdin() -> &'static str {
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED
        .get_or_init(|| {
            // Strip `;` comments (CL line-comment syntax) and join on
            // spaces.  The Lisp file contains no `;` inside strings,
            // so the naive split is safe — if that ever changes, this
            // needs a real lexer.
            let stripped: String = AX_PLOTTING_LISP
                .lines()
                .map(|line| line.split(';').next().unwrap_or("").trim_end())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            format!(":lisp (progn {})\n", stripped)
        })
        .as_str()
}

/// Returns the Maxima code to be evaluated during session init.
pub fn plotting_init_code() -> &'static str {
    AX_PLOTTING_MAC
}
