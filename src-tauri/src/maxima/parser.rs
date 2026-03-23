use regex::Regex;
use std::fs;
use std::path::Path;

use crate::catalog::search::Catalog;
use crate::maxima::backend::Backend;
use crate::maxima::errors;
use crate::maxima::types::EvalResult;

/// LaTeX values that indicate tex(%) had no real result (e.g. after an error)
const JUNK_LATEX: &[&str] = &[
    "\\mathbf{false}",
    "\\mathit{false}",
    "\\it false",
    "false",
    "\\mathit{\\%}",
    "\\it \\%",
    "0",
];

fn is_junk_latex(inner: &str) -> bool {
    JUNK_LATEX.iter().any(|j| inner == *j)
        || inner.contains("__AXIMAR_")
        || inner.contains("AXIMAR")
        // String results: tex() wraps them in \mbox{...} which KaTeX can't
        // render well — fall back to text output instead
        || (inner.starts_with("\\mbox{") && inner.ends_with('}'))
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

pub fn parse_output(cell_id: &str, lines: &[String], duration_ms: u64, catalog: &Catalog, backend: &Backend) -> EvalResult {
    let label_re = Regex::new(r"__AXIMAR_LABEL__\s+(\d+)").unwrap();
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

    for line in lines {
        let trimmed = line.trim();

        // Accumulate multi-line LaTeX: started with $$ but not yet closed
        if let Some(ref mut buf) = latex_buf {
            buf.push_str(trimmed);
            if trimmed.ends_with("$$") {
                // Complete LaTeX block — strip $$ delimiters
                let full = buf.clone();
                latex_buf = None;
                let inner = &full[2..full.len() - 2];
                if !is_junk_latex(inner) {
                    latex = Some(inner.to_string());
                }
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

        // Skip "false" after LaTeX line (return value of tex())
        if skip_next_false {
            skip_next_false = false;
            if trimmed == "false" {
                continue;
            }
        }

        // Check for LaTeX output (may be single-line or start of multi-line)
        if trimmed.starts_with("$$") {
            if trimmed.ends_with("$$") && trimmed.len() > 4 {
                // Single-line LaTeX
                let inner = &trimmed[2..trimmed.len() - 2];
                if !is_junk_latex(inner) {
                    latex = Some(inner.to_string());
                }
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

    // If there's an error, don't show text_lines (they may be noise)
    let text_output = if has_error {
        String::new()
    } else {
        text_lines.join("\n")
    };

    // If error, discard any junk latex that came from tex(%) on the error state
    let latex = if has_error { None } else { latex };

    // Detect SVG plot file paths in text output: Maxima returns e.g. ["/tmp/maxplot.svg"]
    let svg_path_re = Regex::new(r#"\[?"([^"]+\.svg)"(?:,\s*"[^"]+\.svg")*\]?"#).unwrap();
    let mut plot_svg: Option<String> = None;
    let text_output = if !has_error {
        if let Some(caps) = svg_path_re.captures(&text_output) {
            let raw_svg_path = caps[1].to_string();
            // Translate path for Docker/WSL backends, or use as-is for Local
            let svg_path = backend
                .translate_svg_path(&raw_svg_path)
                .unwrap_or(raw_svg_path);
            if is_safe_svg_path(&svg_path, backend) {
                if let Ok(svg_content) = fs::read_to_string(&svg_path) {
                    plot_svg = Some(svg_content);
                }
            }
            // Strip the file path line from text output
            let cleaned: Vec<&str> = text_output
                .lines()
                .filter(|line| !svg_path_re.is_match(line))
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

    let error_info = error
        .as_ref()
        .and_then(|e| errors::enhance_error(e, catalog));

    // Don't expose an output label for errors (no %oN was assigned)
    let output_label = if has_error { None } else { output_label };

    EvalResult {
        cell_id: cell_id.to_string(),
        text_output,
        latex,
        plot_svg,
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
}
