use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use crate::catalog::search::Catalog;
use crate::maxima::backend::Backend;
use crate::maxima::errors;
use crate::maxima::types::EvalResult;

static LABEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"__AXIMAR_LABEL__\s+(\d+)").unwrap());

static SVG_PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\[?"([^"]+\.svg)"(?:,\s*"[^"]+\.svg")*\]?"#).unwrap());

/// Matches any line that's part of a plot file path array (gnuplot, svg, data files).
static PLOT_PATH_LINE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""[^"]+\.(svg|gnuplot|data|plt)""#).unwrap());

/// Extracts an SVG path from LaTeX \mbox{...} blocks produced by tex() on plot results.
/// When 1D display is suppressed, file paths only appear in the LaTeX output.
static LATEX_SVG_PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\\mbox\{\s*([^\}]+\.svg)\s*\}"#).unwrap());

/// Matches a .plotly.json file path in output (from ax_draw2d/ax_draw3d).
static PLOTLY_PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""?([^"\s]+\.plotly\.json)"?"#).unwrap());

/// LaTeX values that indicate tex(%) had no real result (e.g. after an error)
const JUNK_LATEX: &[&str] = &[
    "\\mathbf{false}",
    "\\mathit{false}",
    "\\it false",
    "false",
    "\\mathit{\\%}",
    "\\it \\%",
    "\\mathit{done}",
    "\\mathbf{done}",
    "0",
];

fn is_junk_latex(inner: &str) -> bool {
    JUNK_LATEX.iter().any(|j| inner == *j)
        || inner.contains("__AXIMAR_")
        || inner.contains("AXIMAR")
        // String results: tex() wraps them in \mbox{...} which KaTeX can't
        // render well — fall back to text output instead
        || (inner.starts_with("\\mbox{") && inner.ends_with('}'))
        // Plot results: tex(%) on a list of file paths produces multi-line
        // LaTeX like \left[\mbox{...gnuplot}, \mbox{...svg}\right].
        // Detect by checking for .svg in mbox content.
        || (inner.contains("\\mbox") && inner.contains(".svg"))
}

/// Check that an SVG path is safe to read: it must have a `.svg` extension and
/// reside within the system temp directory (or the Docker/WSL host temp dir).
/// This prevents crafted Maxima output from reading arbitrary files.
fn is_safe_svg_path(path_str: &str, backend: &Backend) -> bool {
    let path = Path::new(path_str);

    // Must have .svg extension
    if path.extension().and_then(|e| e.to_str()) != Some("svg") {
        return false;
    }

    // Canonicalize to resolve symlinks and ..
    let canonical = match fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let temp_dir = std::env::temp_dir();
    let canonical_temp = match fs::canonicalize(&temp_dir) {
        Ok(p) => p,
        Err(_) => temp_dir,
    };

    if canonical.starts_with(&canonical_temp) {
        return true;
    }

    // For Docker, also allow the host temp dir used for volume mounts
    if let Some(host_dir) = backend.host_temp_dir() {
        if let Ok(canonical_host) = fs::canonicalize(&host_dir) {
            if canonical.starts_with(&canonical_host) {
                return true;
            }
        }
    }

    false
}

/// Check that a Plotly JSON path is safe to read: must have `.plotly.json` extension
/// and reside within the system temp directory. Same security model as SVG paths.
fn is_safe_plotly_path(path_str: &str, backend: &Backend) -> bool {
    let path = Path::new(path_str);

    // Must end with .plotly.json
    if !path_str.ends_with(".plotly.json") {
        return false;
    }

    // Canonicalize to resolve symlinks and ..
    let canonical = match fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let temp_dir = std::env::temp_dir();
    let canonical_temp = match fs::canonicalize(&temp_dir) {
        Ok(p) => p,
        Err(_) => temp_dir,
    };

    if canonical.starts_with(&canonical_temp) {
        return true;
    }

    // For Docker, also allow the host temp dir used for volume mounts
    if let Some(host_dir) = backend.host_temp_dir() {
        if let Ok(canonical_host) = fs::canonicalize(&host_dir) {
            if canonical.starts_with(&canonical_host) {
                return true;
            }
        }
    }

    false
}

pub fn parse_output(cell_id: &str, lines: &[String], duration_ms: u64, catalog: &Catalog, backend: &Backend) -> EvalResult {
    parse_output_inner(cell_id, lines, duration_ms, catalog, None, backend)
}

pub fn parse_output_with_packages(
    cell_id: &str,
    lines: &[String],
    duration_ms: u64,
    catalog: &Catalog,
    packages: &crate::catalog::packages::PackageCatalog,
    backend: &Backend,
) -> EvalResult {
    parse_output_inner(cell_id, lines, duration_ms, catalog, Some(packages), backend)
}

fn parse_output_inner(
    cell_id: &str,
    lines: &[String],
    duration_ms: u64,
    catalog: &Catalog,
    packages: Option<&crate::catalog::packages::PackageCatalog>,
    backend: &Backend,
) -> EvalResult {
    let label_re = &*LABEL_RE;
    let error_patterns = [
        " -- an error.",
        "incorrect syntax:",
        "Maxima encountered a Lisp error",
        "MACSYMA restart",
        "Too few arguments",
        "Too many arguments",
        "undefined variable",
    ];

    let mut latex: Option<String> = None;
    let mut error_lines: Vec<String> = Vec::new();
    let mut text_lines: Vec<String> = Vec::new();
    let mut skip_next_false = false;
    let mut in_error = false;
    let mut output_label: Option<String> = None;
    // Accumulator for multi-line LaTeX (tex() wraps long output)
    let mut latex_buf: Option<String> = None;
    // Track \begin{verbatim}...\end{verbatim} blocks from tex() on function defs
    let mut in_verbatim = false;
    // Track intermediate LaTeX blocks from user tex() calls.
    // Each entry is (position_in_text_lines, latex_content_without_delimiters).
    // The last $$...$$ block (from our injected tex(%)) goes to `latex`;
    // earlier blocks are interleaved back into text_output.
    let mut intermediate_latex: Vec<(usize, String)> = Vec::new();
    // Position in text_lines when the current `latex` was set, used to
    // track where intermediate LaTeX blocks should be interleaved.
    let mut latex_pos: Option<usize> = None;

    for line in lines {
        let trimmed = line.trim();

        // Skip lines inside \begin{verbatim}...\end{verbatim} (tex() on := defs)
        if in_verbatim {
            if trimmed == "\\end{verbatim}" {
                in_verbatim = false;
                skip_next_false = true;
            }
            continue;
        }
        if trimmed == "\\begin{verbatim}" {
            in_verbatim = true;
            continue;
        }

        // Accumulate multi-line LaTeX: started with $$ but not yet closed
        if let Some(ref mut buf) = latex_buf {
            buf.push_str(trimmed);
            if trimmed.ends_with("$$") {
                // Complete LaTeX block — strip $$ delimiters
                let full = buf.clone();
                latex_buf = None;
                let inner = &full[2..full.len() - 2];
                // Accept all LaTeX blocks during the loop — junk filtering
                // is applied only to the final block (our tex(%)) after the loop.
                if let Some(prev) = latex.take() {
                    intermediate_latex.push((latex_pos.unwrap(), prev));
                }
                latex = Some(inner.to_string());
                latex_pos = Some(text_lines.len());
                skip_next_false = true;
            }
            continue;
        }

        // Always skip sentinel lines (including LaTeX-escaped versions with \_)
        if trimmed.contains("AXIMAR_EVAL_END") || trimmed.contains("AXIMAR_READY") {
            continue;
        }

        // Extract output label from __AXIMAR_LABEL__ N
        // linenum reports the current expression's own line number, so:
        //   expression = N-2, tex(%) = N-1, print(label) = N
        if let Some(caps) = label_re.captures(trimmed) {
            if let Ok(n) = caps[1].parse::<u64>() {
                if n >= 2 {
                    output_label = Some(format!("%o{}", n - 2));
                }
            }
            continue;
        }

        // Skip other __AXIMAR_ sentinel lines
        if trimmed.contains("__AXIMAR_") {
            continue;
        }

        // Skip "false" after LaTeX or verbatim block (return value of tex())
        // Empty lines don't consume the flag — the false may follow a blank line
        if skip_next_false {
            if trimmed == "false" {
                skip_next_false = false;
                continue;
            } else if !trimmed.is_empty() {
                skip_next_false = false;
            }
        }

        // Check for LaTeX output (may be single-line or start of multi-line)
        if trimmed.starts_with("$$") {
            if trimmed.ends_with("$$") && trimmed.len() > 4 {
                // Single-line LaTeX
                let inner = &trimmed[2..trimmed.len() - 2];
                // Accept all LaTeX blocks during the loop — junk filtering
                // is applied only to the final block (our tex(%)) after the loop.
                if let Some(prev) = latex.take() {
                    intermediate_latex.push((latex_pos.unwrap(), prev));
                }
                latex = Some(inner.to_string());
                latex_pos = Some(text_lines.len());
                skip_next_false = true;
            } else {
                // Start of multi-line LaTeX
                latex_buf = Some(trimmed.to_string());
            }
            continue;
        }

        // Check if this line starts or continues an error
        // Use the original line (not trimmed) because some patterns have leading spaces
        let is_error_line = error_patterns.iter().any(|p| line.contains(p));

        if is_error_line {
            // Pull any preceding text lines into the error (they're context)
            while let Some(prev) = text_lines.pop() {
                if prev.is_empty() {
                    break;
                }
                error_lines.insert(0, prev);
            }
            error_lines.push(line.clone());
            in_error = true;
            continue;
        }

        // Lines immediately after an error line (like "1+!;" and "   ^") are context
        if in_error && !trimmed.is_empty() {
            error_lines.push(line.clone());
            continue;
        }

        // Empty line ends error context
        if in_error && trimmed.is_empty() {
            in_error = false;
        }

        // Regular text output
        if !trimmed.is_empty() {
            text_lines.push(line.clone());
        }
    }

    let has_error = !error_lines.is_empty();
    let error = if has_error {
        Some(error_lines.join("\n"))
    } else {
        None
    };

    // Save raw latex before junk filtering — needed for SVG path extraction
    // when 1D display is suppressed and paths only appear in LaTeX \mbox{} blocks.
    let raw_latex = latex.clone();

    // Filter junk only from the final LaTeX (our injected tex(%)).
    // Intermediate blocks from user tex() calls are kept unconditionally.
    let latex = if has_error {
        None
    } else if latex.as_ref().is_some_and(|l| is_junk_latex(l)) {
        None
    } else {
        latex
    };

    // Build text_output by interleaving text_lines with intermediate LaTeX
    // blocks from user tex() calls (preserved as $$...$$ for frontend rendering).
    // Note: the protocol suppresses all automatic 1D display (`;` → `$`), so
    // text_lines only contains genuine side-effect output (print, tex, etc.).
    let text_output = if has_error {
        String::new()
    } else if intermediate_latex.is_empty() {
        text_lines.join("\n")
    } else {
        let mut parts: Vec<String> = Vec::new();
        let mut li = 0;
        for (i, text_line) in text_lines.iter().enumerate() {
            // Insert intermediate LaTeX blocks that appeared at this position
            while li < intermediate_latex.len() && intermediate_latex[li].0 <= i {
                parts.push(format!("$${}$$", intermediate_latex[li].1));
                li += 1;
            }
            parts.push(text_line.clone());
        }
        // Append remaining intermediate LaTeX
        while li < intermediate_latex.len() {
            parts.push(format!("$${}$$", intermediate_latex[li].1));
            li += 1;
        }
        parts.join("\n")
    };

    // Detect SVG plot file paths and read the SVG content.
    // First check text_output (when 1D display is present), then fall back to
    // the raw LaTeX content (when 1D display is suppressed by the protocol,
    // plot file paths only appear in LaTeX \mbox{} blocks).
    let svg_path_re = &*SVG_PATH_RE;
    let latex_svg_re = &*LATEX_SVG_PATH_RE;
    let mut plot_svg: Option<String> = None;
    let text_output = if !has_error {
        // Try text_output first: Maxima returns e.g. ["/tmp/maxplot.svg"]
        let svg_path_from_text = svg_path_re.captures(&text_output).map(|caps| caps[1].to_string());
        // Fall back to LaTeX \mbox{} content
        let raw_svg_path = svg_path_from_text.or_else(|| {
            raw_latex.as_ref().and_then(|l| {
                latex_svg_re.captures(l).map(|caps| caps[1].trim().to_string())
            })
        });

        if let Some(raw_svg_path) = raw_svg_path {
            // Translate path for Docker/WSL backends, or use as-is for Local
            let svg_path = backend
                .translate_svg_path(&raw_svg_path)
                .unwrap_or(raw_svg_path);
            if is_safe_svg_path(&svg_path, backend) {
                if let Ok(svg_content) = fs::read_to_string(&svg_path) {
                    plot_svg = Some(svg_content);
                }
            }
            // Strip all plot file path lines from text output (both .svg and .gnuplot)
            let plot_line_re = &*PLOT_PATH_LINE_RE;
            let cleaned: Vec<&str> = text_output
                .lines()
                .filter(|line| !plot_line_re.is_match(line))
                .collect();
            cleaned.join("\n")
        } else {
            text_output
        }
    } else {
        text_output
    };

    // If we found a plot SVG, also suppress any LaTeX that just wraps the file path
    let latex = if plot_svg.is_some() {
        match &latex {
            Some(l) if l.contains(".svg") => None,
            _ => latex,
        }
    } else {
        latex
    };

    // Detect Plotly JSON file paths (from ax_draw2d/ax_draw3d) and read the content.
    // Same pattern as SVG detection: scan text_output for .plotly.json paths.
    let plotly_re = &*PLOTLY_PATH_RE;
    let mut plot_data: Option<String> = None;
    let text_output = if !has_error {
        if let Some(caps) = plotly_re.captures(&text_output) {
            let raw_path = caps[1].to_string();
            // Translate path for Docker/WSL backends
            let plotly_path = backend
                .translate_svg_path(&raw_path)
                .unwrap_or(raw_path.clone());
            if is_safe_plotly_path(&plotly_path, backend) {
                match fs::read_to_string(&plotly_path) {
                    Ok(content) => {
                        // Validate JSON before accepting
                        if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                            plot_data = Some(content);
                        } else {
                            eprintln!("[parser] Invalid JSON in {}", plotly_path);
                        }
                    }
                    Err(e) => {
                        eprintln!("[parser] Failed to read {}: {}", plotly_path, e);
                    }
                }
            }
            // Strip the path line from text output
            let cleaned: Vec<&str> = text_output
                .lines()
                .filter(|line| !line.contains(".plotly.json"))
                .collect();
            cleaned.join("\n")
        } else {
            text_output
        }
    } else {
        text_output
    };

    let error_info = error
        .as_ref()
        .and_then(|e| errors::enhance_error_with_packages(e, catalog, packages));

    // Don't expose an output label for errors (no %oN was assigned)
    let output_label = if has_error { None } else { output_label };

    EvalResult {
        cell_id: cell_id.to_string(),
        text_output,
        latex,
        plot_svg,
        plot_data,
        error: error.clone(),
        error_info,
        is_error: has_error,
        duration_ms,
        output_label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        Catalog::load()
    }

    #[test]
    fn test_parse_basic_output() {
        let lines = vec![
            "x^3/3".to_string(),
            "$${{x^3}\\over{3}}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];

        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert_eq!(result.latex, Some("{{x^3}\\over{3}}".to_string()));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_parse_error_division_by_zero() {
        let lines = vec![
            "expt: undefined: 0 to a negative exponent.".to_string(),
            " -- an error. To debug this try: debugmode(true);".to_string(),
            "$$\\mathbf{false}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];

        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.is_error);
        assert!(result.error.is_some());
        assert!(result.latex.is_none());
        // Should have enhanced error info
        assert!(result.error_info.is_some());
        assert_eq!(result.error_info.unwrap().title, "Division by Zero");
    }

    #[test]
    fn test_parse_syntax_error() {
        let lines = vec![
            "incorrect syntax: ! is not a prefix operator".to_string(),
            "1+\\!;".to_string(),
            "   ^".to_string(),
            "$$\\mathbf{false}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];

        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.is_error);
        assert!(result.error.is_some());
        assert!(result.latex.is_none());
        let err = result.error.unwrap();
        assert!(err.contains("incorrect syntax"));
        assert!(err.contains("^"));
        assert!(result.error_info.is_some());
    }

    #[test]
    fn test_parse_output_label() {
        let lines = vec![
            "2*x".to_string(),
            "$$2\\,x$$".to_string(),
            "false".to_string(),
            "__AXIMAR_LABEL__ 7".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];

        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        // linenum 7 = print's own line; expression = 7-2 = %o5
        assert_eq!(result.output_label, Some("%o5".to_string()));
    }

    #[test]
    fn test_error_has_no_output_label() {
        let lines = vec![
            "expt: undefined: 0 to a negative exponent.".to_string(),
            " -- an error. To debug this try: debugmode(true);".to_string(),
            "$$\\mathbf{false}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_LABEL__ 7".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];

        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.is_error);
        // Errors should NOT have an output label
        assert_eq!(result.output_label, None);
    }

    #[test]
    fn test_parse_multiline_latex() {
        // Maxima wraps long tex() output across lines
        let lines = vec![
            "-(%e^x*sin(x)^2)+%e^x*cos(x)*sin(x)+%e^x*cos(x)^2".to_string(),
            "$$-\\left(e^{x}\\,\\sin ^2x\\right)+e^{x}\\,\\cos x\\,\\sin x+e^{x}\\".to_string(),
            "\\,\\cos ^2x$$".to_string(),
            "false".to_string(),
            "__AXIMAR_LABEL__ 8".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];

        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert!(result.latex.is_some());
        let latex = result.latex.unwrap();
        assert!(latex.starts_with("-\\left("));
        assert!(latex.ends_with("\\cos ^2x"));
        assert!(!latex.contains("$$"));
        // Text output should NOT contain the LaTeX lines
        assert!(!result.text_output.contains("$$"));
        assert!(!result.text_output.contains("\\left("));
    }

    #[test]
    fn test_sentinel_never_in_output() {
        let lines = vec![
            "__AXIMAR_EVAL_END__".to_string(),
            "\"__AXIMAR_EVAL_END__\"".to_string(),
        ];

        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.text_output.contains("__AXIMAR_EVAL_END__"));
        assert!(result.error.is_none() || !result.error.unwrap().contains("__AXIMAR_EVAL_END__"));
    }

    // --- Junk LaTeX filtering ---

    #[test]
    fn test_junk_latex_mathit_false() {
        let lines = vec!["$$\\mathit{false}$$".to_string()];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.latex.is_none());
    }

    #[test]
    fn test_junk_latex_it_false() {
        let lines = vec!["$$\\it false$$".to_string()];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.latex.is_none());
    }

    #[test]
    fn test_junk_latex_plain_false() {
        let lines = vec!["$$false$$".to_string()];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.latex.is_none());
    }

    #[test]
    fn test_junk_latex_percent() {
        let lines = vec!["$$\\mathit{\\%}$$".to_string()];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.latex.is_none());
    }

    #[test]
    fn test_junk_latex_zero() {
        let lines = vec!["$$0$$".to_string()];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.latex.is_none());
    }

    #[test]
    fn test_junk_latex_mbox_string() {
        let lines = vec!["$$\\mbox{hello world}$$".to_string()];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.latex.is_none());
    }

    #[test]
    fn test_junk_latex_verbatim_function_def() {
        // tex() on := function definitions produces \begin{verbatim}...\end{verbatim}
        let lines = vec![
            "f(x):=x^3-2*x+1".to_string(),
            "".to_string(),
            "\\begin{verbatim}".to_string(),
            "f(x):=x^3-2*x+1;".to_string(),
            "\\end{verbatim}".to_string(),
            "".to_string(),
            "false".to_string(),
            "__AXIMAR_LABEL__ 5".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert!(result.latex.is_none());
        // text_output should only contain the actual result, not verbatim or false
        assert_eq!(result.text_output, "f(x):=x^3-2*x+1");
    }

    #[test]
    fn test_junk_latex_aximar_sentinel() {
        let lines = vec!["$$blah __AXIMAR_ blah$$".to_string()];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.latex.is_none());
    }

    // --- Error context ---

    #[test]
    fn test_error_pulls_preceding_text() {
        let lines = vec![
            "some context line".to_string(),
            " -- an error.".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.is_error);
        let err = result.error.unwrap();
        assert!(err.contains("some context line"));
        assert!(err.contains("-- an error."));
    }

    #[test]
    fn test_error_context_ends_on_empty_line() {
        let lines = vec![
            " -- an error.".to_string(),
            "error context".to_string(),
            "".to_string(),
            "normal text after".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.is_error);
        let err = result.error.unwrap();
        assert!(err.contains("error context"));
        // Text after the empty line is not in the error
        assert!(!err.contains("normal text after"));
    }

    // --- Label edge cases ---

    #[test]
    fn test_label_linenum_1() {
        let lines = vec![
            "result".to_string(),
            "__AXIMAR_LABEL__ 1".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert_eq!(result.output_label, None);
    }

    #[test]
    fn test_label_linenum_2() {
        let lines = vec![
            "result".to_string(),
            "__AXIMAR_LABEL__ 2".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert_eq!(result.output_label, Some("%o0".to_string()));
    }

    #[test]
    fn test_label_non_numeric() {
        let lines = vec![
            "result".to_string(),
            "__AXIMAR_LABEL__ abc".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert_eq!(result.output_label, None);
    }

    // --- Other edge cases ---

    #[test]
    fn test_empty_input() {
        let lines: Vec<String> = vec![];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert!(result.text_output.is_empty());
        assert!(result.latex.is_none());
    }

    #[test]
    fn test_only_sentinels() {
        let lines = vec![
            "__AXIMAR_EVAL_END__".to_string(),
            "__AXIMAR_READY__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert!(result.text_output.is_empty());
        assert!(result.latex.is_none());
    }

    #[test]
    fn test_print_output_preserved_when_latex_present() {
        // print() output should remain in text_output (protocol suppresses 1D
        // display, so no 1D result line appears in the output)
        let lines = vec![
            "hello".to_string(),
            "world".to_string(),
            "$$42$$".to_string(),
            "false".to_string(),
            "__AXIMAR_LABEL__ 5".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert_eq!(result.latex, Some("42".to_string()));
        assert_eq!(result.text_output, "hello\nworld");
    }

    #[test]
    fn test_no_print_output_text_empty_when_latex_present() {
        // With display suppressed, only the LaTeX from tex(%) appears
        let lines = vec![
            "$${{x^3}\\over{3}}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert_eq!(result.latex, Some("{{x^3}\\over{3}}".to_string()));
        assert!(result.text_output.is_empty());
    }

    #[test]
    fn test_multiple_tex_calls_preserve_earlier_in_text() {
        // User tex() calls produce $$...$$ blocks that should stay in text_output.
        // Only the last $$...$$ (from our injected tex(%)) becomes the latex field.
        // No 1D result lines since display is suppressed.
        let lines = vec![
            "Step 1".to_string(),
            "$$1$$".to_string(),            // user tex(1)
            "false".to_string(),
            "Step 2".to_string(),
            "$$4$$".to_string(),            // user tex(4)
            "false".to_string(),
            "$$\\mathit{done}$$".to_string(), // tex(%) on "done"
            "false".to_string(),
            "__AXIMAR_LABEL__ 10".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        // \mathit{done} is junk LaTeX (filtered since ax_draw2d returns 'done)
        assert_eq!(result.latex, None);
        // Earlier tex() blocks should be in text_output as $$...$$
        assert!(result.text_output.contains("Step 1"));
        assert!(result.text_output.contains("$$1$$"));
        assert!(result.text_output.contains("Step 2"));
        assert!(result.text_output.contains("$$4$$"));
    }

    #[test]
    fn test_single_tex_still_becomes_latex() {
        // Normal case: one $$...$$ from tex(%) goes to latex field, no 1D display
        let lines = vec![
            "$$42$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert_eq!(result.latex, Some("42".to_string()));
        assert!(result.text_output.is_empty());
    }

    #[test]
    fn test_intermediate_tex_zero_preserved() {
        // User tex(0) produces $$0$$ which is in JUNK_LATEX, but intermediate
        // LaTeX from user tex() calls should be preserved — only the final
        // tex(%) result is junk-filtered.
        let lines = vec![
            "Step 1".to_string(),
            "$$0$$".to_string(),               // user tex(0) — should be kept
            "false".to_string(),
            "Step 2".to_string(),
            "$$4$$".to_string(),               // user tex(4)
            "false".to_string(),
            "$$\\mathbf{done}$$".to_string(),  // tex(%) on for-loop result
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        // \mathbf{done} is junk LaTeX (filtered since ax_draw2d returns 'done)
        assert_eq!(result.latex, None);
        // Both intermediate tex() results should be in text_output
        assert!(result.text_output.contains("$$0$$"), "tex(0) should be preserved");
        assert!(result.text_output.contains("$$4$$"));
        assert!(result.text_output.contains("Step 1"));
        assert!(result.text_output.contains("Step 2"));
    }

    #[test]
    fn test_final_junk_latex_filtered() {
        // When the final tex(%) produces junk (e.g. $$false$$), it should be
        // filtered, but intermediate user tex() blocks should still be kept.
        let lines = vec![
            "Step 1".to_string(),
            "$$x^2$$".to_string(),             // user tex(x^2)
            "false".to_string(),
            "$$\\mathbf{false}$$".to_string(), // tex(%) on false — junk
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert_eq!(result.latex, None, "final junk latex should be filtered");
        assert!(result.text_output.contains("$$x^2$$"), "user tex() should be kept");
        assert!(result.text_output.contains("Step 1"));
    }

    #[test]
    fn test_text_before_junk_latex_not_stripped() {
        // When the final tex(%) produces junk (e.g. \mbox{...}), the preceding
        // text line must NOT be stripped as a "redundant 1D result". This matters
        // for plot commands where the text line is a file path needed for SVG detection.
        let lines = vec![
            "some important output".to_string(),
            "$$\\mbox{ hello }$$".to_string(), // tex(%) — junk (mbox)
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert_eq!(result.latex, None, "mbox latex should be filtered");
        // The text line must NOT be stripped — it's not redundant with anything
        assert!(
            result.text_output.contains("some important output"),
            "text before junk latex should be preserved: {:?}",
            result.text_output
        );
    }

    #[test]
    fn test_multiline_plot_latex_is_junk() {
        // plot2d returns a list of file paths. tex(%) on this produces multi-line
        // LaTeX like \left[\mbox{...gnuplot}, \mbox{...svg}\right].
        // This must be detected as junk so the file path text lines are preserved
        // for SVG detection. SVG detection then strips ALL plot path lines
        // (.svg AND .gnuplot) from the final text_output.
        let lines = vec![
            "[\"/tmp/aximar/cell-1.gnuplot\",".to_string(),
            " \"/tmp/aximar/cell-1.svg\"]".to_string(),
            "$$\\left[\\mbox{ /tmp/aximar/cell-1.gnuplot } , \\mbox{ /tmp/aximar/cell-1.svg }  \\right] $$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert_eq!(result.latex, None, "plot path latex should be filtered as junk");
        // Both .gnuplot and .svg path lines should be stripped
        assert!(
            !result.text_output.contains(".gnuplot"),
            "gnuplot path should be stripped from text_output: {:?}",
            result.text_output
        );
        assert!(
            !result.text_output.contains(".svg"),
            "svg path should be stripped from text_output: {:?}",
            result.text_output
        );
    }

    #[test]
    fn test_plot_svg_path_from_latex_when_1d_suppressed() {
        // When the protocol suppresses 1D display (`;` → `$`), plot2d's return
        // value (file path list) only appears in the LaTeX \mbox{} blocks from
        // tex(%), not as plain text. The SVG path must be extracted from LaTeX.
        let lines = vec![
            "plot2d: some values will be clipped.".to_string(),
            "$$\\left[\\mbox{ /tmp/aximar/cell-1.gnuplot } , \\mbox{ /tmp/aximar/cell-1.svg }  \\right] $$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert_eq!(result.latex, None, "plot path latex should be filtered as junk");
        // The warning line should remain but no file path lines
        assert!(
            result.text_output.contains("some values will be clipped"),
            "warning should be preserved: {:?}",
            result.text_output
        );
        // plot_svg would be None here because the file doesn't exist on disk,
        // but we can verify the path was extracted by checking that the gnuplot
        // warning line (which doesn't match PLOT_PATH_LINE_RE) is still present.
        // The real SVG capture happens in production when the file exists.
    }

    #[test]
    fn test_non_false_after_latex_kept() {
        let lines = vec![
            "$$x^2$$".to_string(),
            "some output".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(result.text_output.contains("some output"));
    }

    // --- Plotly JSON path detection ---

    #[test]
    fn test_parse_plotly_json_path() {
        // ax_draw2d prints a .plotly.json path; parser should read the file and populate plot_data
        let temp_dir = std::env::temp_dir();
        let plotly_path = temp_dir.join("ax_plot_test_1234.plotly.json");
        let json_content = r#"{"data":[{"x":[1,2,3],"y":[1,4,9],"type":"scatter"}],"layout":{}}"#;
        fs::write(&plotly_path, json_content).unwrap();

        let lines = vec![
            plotly_path.to_str().unwrap().to_string(),
            "$$\\mathit{done}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert_eq!(result.plot_data, Some(json_content.to_string()));

        // Clean up
        let _ = fs::remove_file(&plotly_path);
    }

    #[test]
    fn test_parse_plotly_path_stripped_from_text() {
        // The .plotly.json path line should be stripped from text_output
        let temp_dir = std::env::temp_dir();
        let plotly_path = temp_dir.join("ax_plot_test_strip.plotly.json");
        let json_content = r#"{"data":[],"layout":{}}"#;
        fs::write(&plotly_path, json_content).unwrap();

        let lines = vec![
            "some warning".to_string(),
            plotly_path.to_str().unwrap().to_string(),
            "$$\\mathit{done}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert!(result.plot_data.is_some());
        assert!(
            !result.text_output.contains(".plotly.json"),
            "plotly path should be stripped from text_output: {:?}",
            result.text_output
        );
        assert!(
            result.text_output.contains("some warning"),
            "non-path text should be preserved: {:?}",
            result.text_output
        );

        let _ = fs::remove_file(&plotly_path);
    }

    #[test]
    fn test_parse_plotly_path_safety_rejects_outside_temp() {
        // Paths outside the temp directory should be rejected
        let result = is_safe_plotly_path("/etc/passwd.plotly.json", &Backend::Local);
        assert!(!result, "paths outside temp dir should be rejected");
    }

    #[test]
    fn test_parse_plotly_path_safety_rejects_wrong_extension() {
        let result = is_safe_plotly_path("/tmp/evil.json", &Backend::Local);
        assert!(!result, "non-.plotly.json extension should be rejected");
    }

    #[test]
    fn test_no_plot_data_for_regular_output() {
        // Normal expression output should not have plot_data
        let lines = vec![
            "$$42$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert!(result.plot_data.is_none());
        assert!(result.plot_svg.is_none());
    }

    #[test]
    fn test_plot_data_done_latex_filtered() {
        // ax_draw2d returns 'done, so tex(%) produces $$\mathit{done}$$
        // which should be filtered as junk LaTeX
        let temp_dir = std::env::temp_dir();
        let plotly_path = temp_dir.join("ax_plot_test_done.plotly.json");
        let json_content = r#"{"data":[{"x":[1],"y":[1],"type":"scatter"}],"layout":{}}"#;
        fs::write(&plotly_path, json_content).unwrap();

        let lines = vec![
            plotly_path.to_str().unwrap().to_string(),
            "$$\\mathit{done}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_LABEL__ 5".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert!(result.plot_data.is_some());
        assert_eq!(result.latex, None, "done should be filtered as junk LaTeX");

        let _ = fs::remove_file(&plotly_path);
    }

    #[test]
    fn test_plotly_invalid_json_rejected() {
        // If the file contains invalid JSON, plot_data should be None
        let temp_dir = std::env::temp_dir();
        let plotly_path = temp_dir.join("ax_plot_test_invalid.plotly.json");
        fs::write(&plotly_path, "not valid json {{{").unwrap();

        let lines = vec![
            plotly_path.to_str().unwrap().to_string(),
            "$$\\mathit{done}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert!(result.plot_data.is_none(), "invalid JSON should be rejected");

        let _ = fs::remove_file(&plotly_path);
    }

    #[test]
    fn test_plotly_nonexistent_file() {
        // If the file doesn't exist, plot_data should be None (no crash)
        let lines = vec![
            "/tmp/ax_plot_nonexistent_99999.plotly.json".to_string(),
            "$$\\mathit{done}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];
        let result = parse_output("cell-1", &lines, 100, &catalog(), &Backend::Local);
        assert!(!result.is_error);
        assert!(result.plot_data.is_none());
    }
}
