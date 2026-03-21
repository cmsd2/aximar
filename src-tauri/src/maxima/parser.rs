use regex::Regex;

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
}

pub fn parse_output(cell_id: &str, lines: &[String], duration_ms: u64) -> EvalResult {
    let latex_re = Regex::new(r"^\$\$.*\$\$$").unwrap();
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

    for line in lines {
        let trimmed = line.trim();

        // Always skip sentinel lines (including LaTeX-escaped versions with \_)
        if trimmed.contains("__AXIMAR_") || trimmed.contains("AXIMAR_EVAL_END") || trimmed.contains("AXIMAR_READY") {
            continue;
        }

        // Skip "false" after LaTeX line (return value of tex())
        if skip_next_false {
            skip_next_false = false;
            if trimmed == "false" {
                continue;
            }
        }

        // Check for LaTeX output
        if latex_re.is_match(trimmed) {
            let inner = &trimmed[2..trimmed.len() - 2];
            // Discard junk LaTeX that appears after errors
            if !is_junk_latex(inner) {
                latex = Some(inner.to_string());
            }
            skip_next_false = true;
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

    EvalResult {
        cell_id: cell_id.to_string(),
        text_output,
        latex,
        plot_svg: None,
        error: error.clone(),
        is_error: has_error,
        duration_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_output() {
        let lines = vec![
            "x^3/3".to_string(),
            "$${{x^3}\\over{3}}$$".to_string(),
            "false".to_string(),
            "__AXIMAR_EVAL_END__".to_string(),
        ];

        let result = parse_output("cell-1", &lines, 100);
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

        let result = parse_output("cell-1", &lines, 100);
        assert!(result.is_error);
        assert!(result.error.is_some());
        assert!(result.latex.is_none());
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

        let result = parse_output("cell-1", &lines, 100);
        assert!(result.is_error);
        assert!(result.error.is_some());
        assert!(result.latex.is_none());
        let err = result.error.unwrap();
        assert!(err.contains("incorrect syntax"));
        assert!(err.contains("^"));
    }

    #[test]
    fn test_sentinel_never_in_output() {
        let lines = vec![
            "__AXIMAR_EVAL_END__".to_string(),
            "\"__AXIMAR_EVAL_END__\"".to_string(),
        ];

        let result = parse_output("cell-1", &lines, 100);
        assert!(!result.text_output.contains("__AXIMAR_EVAL_END__"));
        assert!(result.error.is_none() || !result.error.unwrap().contains("__AXIMAR_EVAL_END__"));
    }
}
