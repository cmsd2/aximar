use regex::Regex;
use std::sync::LazyLock;

use crate::catalog::packages::PackageCatalog;
use crate::maxima::types::EvalResult;
use crate::suggestions::types::Suggestion;

const MAX_SUGGESTIONS: usize = 5;

/// Regex to extract function names from Maxima expressions: `name(`
static FUNC_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b([a-zA-Z_]\w*)\s*\(").unwrap());

pub fn suggestions_for_output(input: &str, output: &EvalResult) -> Vec<Suggestion> {
    suggestions_for_output_with_packages(input, output, None)
}

pub fn suggestions_for_output_with_packages(
    input: &str,
    output: &EvalResult,
    packages: Option<&PackageCatalog>,
) -> Vec<Suggestion> {
    if output.is_error {
        return Vec::new();
    }

    let mut suggestions = Vec::new();
    let input_lower = input.to_lowercase();
    let text = &output.text_output;
    let latex = output.latex.as_deref().unwrap_or("");

    // Check for unevaluated function calls from loadable packages.
    // When Maxima doesn't know a function, it returns it symbolically (e.g.
    // input "pdf_normal(0,0,1)" → output "pdf_normal(0,0,1)").
    if let Some(pkgs) = packages {
        check_unevaluated_package_functions(input, text, pkgs, &mut suggestions);
    }

    // Plot output: only offer plot-specific actions
    if output.plot_svg.is_some() {
        suggestions.push(Suggestion {
            label: "Save SVG".into(),
            template: String::new(),
            description: "Save plot as SVG file".into(),
            action: Some("save_svg".into()),
            position: None,
        });
        return suggestions;
    }

    // After diff(): suggest integrate
    if input_lower.contains("diff(") || input_lower.contains("diff (") {
        suggestions.push(Suggestion {
            label: "Integrate".into(),
            template: "integrate(%, x)".into(),
            description: "Integrate the result".into(),
            action: None,
            position: None,
        });
    }

    // After integrate(): suggest differentiate
    if input_lower.contains("integrate(") || input_lower.contains("integrate (") {
        suggestions.push(Suggestion {
            label: "Differentiate".into(),
            template: "diff(%, x)".into(),
            description: "Differentiate the result".into(),
            action: None,
            position: None,
        });
    }

    // After solve(): suggest extracting values
    if input_lower.contains("solve(") || input_lower.contains("solve (") {
        suggestions.push(Suggestion {
            label: "Extract values".into(),
            template: "map(rhs, %)".into(),
            description: "Extract right-hand sides of solutions".into(),
            action: None,
            position: None,
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
            action: None,
            position: None,
        });
        suggestions.push(Suggestion {
            label: "Eigenvalues".into(),
            template: "eigenvalues(%)".into(),
            description: "Find eigenvalues".into(),
            action: None,
            position: None,
        });
        suggestions.push(Suggestion {
            label: "Inverse".into(),
            template: "invert(%)".into(),
            description: "Compute matrix inverse".into(),
            action: None,
            position: None,
        });
        suggestions.push(Suggestion {
            label: "Transpose".into(),
            template: "transpose(%)".into(),
            description: "Transpose the matrix".into(),
            action: None,
            position: None,
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
            action: None,
            position: None,
        });
        suggestions.push(Suggestion {
            label: "Right-hand side".into(),
            template: "rhs(%)".into(),
            description: "Extract right-hand side".into(),
            action: None,
            position: None,
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
            action: None,
            position: None,
        });
        suggestions.push(Suggestion {
            label: "Trig expand".into(),
            template: "trigexpand(%)".into(),
            description: "Expand trig functions".into(),
            action: None,
            position: None,
        });
    }

    // List in output
    let has_list = text.starts_with('[') || latex.starts_with('[');
    if has_list {
        suggestions.push(Suggestion {
            label: "Length".into(),
            template: "length(%)".into(),
            description: "Count list elements".into(),
            action: None,
            position: None,
        });
        suggestions.push(Suggestion {
            label: "Sort".into(),
            template: "sort(%)".into(),
            description: "Sort the list".into(),
            action: None,
            position: None,
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
            action: None,
            position: None,
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
                action: None,
                position: None,
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
                action: None,
                position: None,
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
                action: None,
                position: None,
            });
        }
    }

    // Deduplicate by template
    let mut seen = std::collections::HashSet::new();
    suggestions.retain(|s| seen.insert(s.template.clone()));

    suggestions.truncate(MAX_SUGGESTIONS);
    suggestions
}

/// Detect unevaluated function calls that come from loadable packages.
///
/// When a function like `pdf_normal(0, 0, 1)` is called without loading its
/// package, Maxima returns it unevaluated. We detect this by checking if any
/// function name from the input also appears unevaluated in the output and
/// belongs to a loadable package.
fn check_unevaluated_package_functions(
    input: &str,
    text_output: &str,
    packages: &PackageCatalog,
    suggestions: &mut Vec<Suggestion>,
) {
    // Collect function names from the input
    let mut seen_packages = std::collections::HashSet::new();

    for caps in FUNC_NAME_RE.captures_iter(input) {
        let func_name = &caps[1];

        // Check if this function name appears in the output text (unevaluated)
        if !text_output.contains(func_name) {
            continue;
        }

        // Check if a package provides this function
        if let Some(pkg_name) = packages.package_for_function(func_name) {
            if seen_packages.insert(pkg_name.to_string()) {
                suggestions.push(Suggestion {
                    label: format!("Load {pkg_name}"),
                    template: format!("load(\"{pkg_name}\")$"),
                    description: format!(
                        "{func_name} is provided by the \"{pkg_name}\" package"
                    ),
                    action: None,
                    position: Some("before".into()),
                });
            }
        }
    }
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

    #[test]
    fn test_plot_svg_suggestions() {
        let mut result = make_result("", None);
        result.plot_svg = Some("<svg>...</svg>".into());
        let suggestions = suggestions_for_output("plot2d(sin(x), [x, -5, 5])", &result);
        assert!(suggestions.iter().any(|s| s.action.as_deref() == Some("save_svg")));
    }

    #[test]
    fn test_unevaluated_package_function_suggests_load() {
        let packages = PackageCatalog::load();
        // pdf_normal is provided by the "distrib" package
        let result = make_result("pdf_normal(0,0,1)", None);
        let suggestions =
            suggestions_for_output_with_packages("pdf_normal(0, 0, 1)", &result, Some(&packages));
        let load_suggestion = suggestions
            .iter()
            .find(|s| s.template.contains("load(\"distrib\")"));
        assert!(
            load_suggestion.is_some(),
            "Expected a suggestion to load distrib, got: {:?}",
            suggestions
        );
    }

    #[test]
    fn test_no_package_suggestion_when_function_evaluated() {
        let packages = PackageCatalog::load();
        // If the output doesn't contain the function name (i.e. it evaluated properly),
        // no package suggestion should appear
        let result = make_result("0.3989422804014327", None);
        let suggestions =
            suggestions_for_output_with_packages("pdf_normal(0, 0, 1)", &result, Some(&packages));
        assert!(
            !suggestions.iter().any(|s| s.template.contains("load(")),
            "Should not suggest loading when function evaluated successfully"
        );
    }
}
