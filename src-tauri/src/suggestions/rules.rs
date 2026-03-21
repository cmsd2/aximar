use regex::Regex;

use crate::maxima::types::EvalResult;
use crate::suggestions::types::Suggestion;

const MAX_SUGGESTIONS: usize = 5;

pub fn suggestions_for_output(input: &str, output: &EvalResult) -> Vec<Suggestion> {
    if output.is_error {
        return Vec::new();
    }

    let mut suggestions = Vec::new();
    let input_lower = input.to_lowercase();
    let text = &output.text_output;
    let latex = output.latex.as_deref().unwrap_or("");

    // After diff(): suggest integrate
    if input_lower.contains("diff(") || input_lower.contains("diff (") {
        suggestions.push(Suggestion {
            label: "Integrate".into(),
            template: "integrate(%, x)".into(),
            description: "Integrate the result".into(),
        });
    }

    // After integrate(): suggest differentiate
    if input_lower.contains("integrate(") || input_lower.contains("integrate (") {
        suggestions.push(Suggestion {
            label: "Differentiate".into(),
            template: "diff(%, x)".into(),
            description: "Differentiate the result".into(),
        });
    }

    // After solve(): suggest extracting values
    if input_lower.contains("solve(") || input_lower.contains("solve (") {
        suggestions.push(Suggestion {
            label: "Extract values".into(),
            template: "map(rhs, %)".into(),
            description: "Extract right-hand sides of solutions".into(),
        });
    }

    // Matrix detected in output
    let has_matrix = latex.contains("pmatrix")
        || latex.contains("\\matrix")
        || text.contains("matrix(")
        || input_lower.contains("matrix(");
    if has_matrix {
        suggestions.push(Suggestion {
            label: "Determinant".into(),
            template: "determinant(%)".into(),
            description: "Compute the determinant".into(),
        });
        suggestions.push(Suggestion {
            label: "Eigenvalues".into(),
            template: "eigenvalues(%)".into(),
            description: "Find eigenvalues".into(),
        });
        suggestions.push(Suggestion {
            label: "Inverse".into(),
            template: "invert(%)".into(),
            description: "Compute matrix inverse".into(),
        });
        suggestions.push(Suggestion {
            label: "Transpose".into(),
            template: "transpose(%)".into(),
            description: "Transpose the matrix".into(),
        });
    }

    // Equation in output (contains =)
    let eq_re = Regex::new(r"[^<>=!]=(?!=)").ok();
    let has_equation = eq_re
        .as_ref()
        .map(|re| re.is_match(text) || re.is_match(latex))
        .unwrap_or(false);
    if has_equation && !input_lower.contains("solve(") {
        suggestions.push(Suggestion {
            label: "Solve".into(),
            template: "solve(%, x)".into(),
            description: "Solve for x".into(),
        });
        suggestions.push(Suggestion {
            label: "Right-hand side".into(),
            template: "rhs(%)".into(),
            description: "Extract right-hand side".into(),
        });
    }

    // Trig in output
    let has_trig = text.contains("sin(")
        || text.contains("cos(")
        || text.contains("tan(")
        || latex.contains("\\sin")
        || latex.contains("\\cos")
        || latex.contains("\\tan");
    if has_trig {
        suggestions.push(Suggestion {
            label: "Trig simplify".into(),
            template: "trigsimp(%)".into(),
            description: "Simplify trig expressions".into(),
        });
        suggestions.push(Suggestion {
            label: "Trig expand".into(),
            template: "trigexpand(%)".into(),
            description: "Expand trig functions".into(),
        });
    }

    // List in output
    let has_list = text.starts_with('[') || latex.starts_with('[');
    if has_list {
        suggestions.push(Suggestion {
            label: "Length".into(),
            template: "length(%)".into(),
            description: "Count list elements".into(),
        });
        suggestions.push(Suggestion {
            label: "Sort".into(),
            template: "sort(%)".into(),
            description: "Sort the list".into(),
        });
    }

    // Numeric result: suggest float conversion
    let has_symbolic_number = latex.contains("\\over")
        || latex.contains("\\sqrt")
        || text.contains("%pi")
        || text.contains("%e")
        || latex.contains("\\pi");
    if has_symbolic_number && !text.contains('.') {
        suggestions.push(Suggestion {
            label: "Float".into(),
            template: "float(%)".into(),
            description: "Convert to decimal".into(),
        });
    }

    // Targeted algebraic suggestions — only when the output structure would benefit
    if !has_matrix && !has_list && !output.text_output.is_empty() {
        // Expand: only when output has products of sums, i.e. contains both * and (
        // or exponents of sums like (a+b)^n
        let worth_expanding = (text.contains('*') && text.contains('('))
            || (text.contains('^') && text.contains('('))
            || latex.contains("\\left(");
        if worth_expanding && !input_lower.contains("expand(") {
            suggestions.push(Suggestion {
                label: "Expand".into(),
                template: "expand(%)".into(),
                description: "Expand products and powers".into(),
            });
        }

        // Factor: only when output has multiple additive terms (polynomials)
        let worth_factoring = text.contains('+')
            || (text.contains('-') && text.len() > 3);
        if worth_factoring && !input_lower.contains("factor(") {
            suggestions.push(Suggestion {
                label: "Factor".into(),
                template: "factor(%)".into(),
                description: "Factor the expression".into(),
            });
        }

        // Simplify: only when output has fractions or nested structure
        let worth_simplifying = text.contains('/')
            || (latex.contains("\\over") && !has_symbolic_number)
            || (text.matches('(').count() >= 2);
        let already_has_simplify = suggestions.iter().any(|s| s.template.contains("simp"));
        if worth_simplifying && !already_has_simplify {
            suggestions.push(Suggestion {
                label: "Simplify".into(),
                template: "ratsimp(%)".into(),
                description: "Simplify the expression".into(),
            });
        }
    }

    // Deduplicate by template
    let mut seen = std::collections::HashSet::new();
    suggestions.retain(|s| seen.insert(s.template.clone()));

    suggestions.truncate(MAX_SUGGESTIONS);
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(text: &str, latex: Option<&str>) -> EvalResult {
        EvalResult {
            cell_id: "test".into(),
            text_output: text.into(),
            latex: latex.map(String::from),
            plot_svg: None,
            error: None,
            error_info: None,
            is_error: false,
            duration_ms: 100,
            output_label: None,
        }
    }

    #[test]
    fn test_after_diff() {
        let result = make_result("3*x^2", Some("3\\,x^2"));
        let suggestions = suggestions_for_output("diff(x^3, x)", &result);
        assert!(suggestions.iter().any(|s| s.template.contains("integrate")));
    }

    #[test]
    fn test_after_integrate() {
        let result = make_result("x^3/3", Some("{{x^3}\\over{3}}"));
        let suggestions = suggestions_for_output("integrate(x^2, x)", &result);
        assert!(suggestions.iter().any(|s| s.template.contains("diff")));
    }

    #[test]
    fn test_matrix_output() {
        let result = make_result(
            "matrix([1,2],[3,4])",
            Some("\\begin{pmatrix}1&2\\cr 3&4\\end{pmatrix}"),
        );
        let suggestions = suggestions_for_output("matrix([1,2],[3,4])", &result);
        assert!(suggestions
            .iter()
            .any(|s| s.template.contains("determinant")));
    }

    #[test]
    fn test_error_no_suggestions() {
        let result = EvalResult {
            cell_id: "test".into(),
            text_output: "".into(),
            latex: None,
            plot_svg: None,
            error: Some("error".into()),
            error_info: None,
            is_error: true,
            duration_ms: 100,
            output_label: None,
        };
        let suggestions = suggestions_for_output("bad(", &result);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_max_suggestions() {
        let result = make_result("sin(x)/x", Some("{{\\sin x}\\over{x}}"));
        let suggestions = suggestions_for_output("something", &result);
        assert!(suggestions.len() <= MAX_SUGGESTIONS);
    }

    #[test]
    fn test_simple_output_no_blanket_suggestions() {
        // Simple results like "5" or "2*x" should NOT get expand/factor/simplify
        let result = make_result("5", Some("5"));
        let suggestions = suggestions_for_output("2+3", &result);
        assert!(!suggestions.iter().any(|s| s.template.contains("expand")));
        assert!(!suggestions.iter().any(|s| s.template.contains("factor")));
        assert!(!suggestions.iter().any(|s| s.template.contains("ratsimp")));
    }

    #[test]
    fn test_expand_suggested_for_products() {
        let result = make_result("(x+1)*(x-1)", Some("\\left(x+1\\right)\\,\\left(x-1\\right)"));
        let suggestions = suggestions_for_output("something", &result);
        assert!(suggestions.iter().any(|s| s.template.contains("expand")));
    }

    #[test]
    fn test_factor_suggested_for_polynomial() {
        let result = make_result("x^2+2*x+1", Some("x^2+2\\,x+1"));
        let suggestions = suggestions_for_output("something", &result);
        assert!(suggestions.iter().any(|s| s.template.contains("factor")));
    }
}
