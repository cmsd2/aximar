/// Embedded Lisp helpers for the plotting functions (loaded first).
///
/// Defines `ax__mktemp` which creates unique temp files via mkstemp.
/// Used by integration tests which write to a file and load via `:lisp (load ...)`.
const AX_PLOTTING_LISP: &str = include_str!("ax_plotting.lisp");

/// The same defun formatted as a `:lisp` command for sending via Maxima stdin.
/// This avoids needing a temp file and works across all backends.
const AX_PLOTTING_LISP_STDIN: &str = ":lisp (defun $ax__mktemp () (multiple-value-bind (fd path) (sb-posix:mkstemp (format nil \"~A/ax_plot_XXXXXX\" $maxima_tempdir)) (sb-posix:close fd) (let ((final (concatenate 'string path \".plotly.json\"))) (rename-file path final) final)))\n";

/// Embedded Maxima code defining ax_plot2d, ax_draw2d, ax_draw3d plotting functions.
///
/// These functions produce Plotly.js JSON specs written to temp files, which the
/// parser detects and reads (same pattern as gnuplot SVG files).
const AX_PLOTTING_MAC: &str = include_str!("ax_plotting.mac");

/// Returns the raw Lisp helper source (for integration tests that write to a file).
pub fn plotting_lisp_code() -> &'static str {
    AX_PLOTTING_LISP
}

/// Returns the Lisp helper as a `:lisp` stdin command for session init.
pub fn plotting_lisp_stdin() -> &'static str {
    AX_PLOTTING_LISP_STDIN
}

/// Returns the Maxima code to be evaluated during session init.
pub fn plotting_init_code() -> &'static str {
    AX_PLOTTING_MAC
}
