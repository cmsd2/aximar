use regex::Regex;

use crate::catalog::search::Catalog;
use crate::maxima::types::ErrorInfo;

pub fn enhance_error(raw_error: &str, catalog: &Catalog) -> Option<ErrorInfo> {
    // Try each pattern in order
    if let Some(info) = check_division_by_zero(raw_error) {
        return Some(info);
    }
    if let Some(info) = check_syntax_error(raw_error) {
        return Some(info);
    }
    if let Some(info) = check_arg_count(raw_error, catalog) {
        return Some(info);
    }
    if let Some(info) = check_undefined_variable(raw_error) {
        return Some(info);
    }
    if let Some(info) = check_undefined_function(raw_error, catalog) {
        return Some(info);
    }
    if let Some(info) = check_lisp_error(raw_error) {
        return Some(info);
    }
    if let Some(info) = check_divergent_integral(raw_error) {
        return Some(info);
    }
    if let Some(info) = check_inconsistent_equations(raw_error) {
        return Some(info);
    }
    if let Some(info) = check_missing_assumption(raw_error) {
        return Some(info);
    }
    if let Some(info) = check_matrix_dimension(raw_error) {
        return Some(info);
    }
    if let Some(info) = check_premature_termination(raw_error) {
        return Some(info);
    }
    if let Some(info) = check_load_failed(raw_error) {
        return Some(info);
    }

    None
}

fn check_division_by_zero(raw: &str) -> Option<ErrorInfo> {
    if raw.contains("0 to a negative exponent") || raw.contains("Division by 0") {
        return Some(ErrorInfo {
            title: "Division by Zero".into(),
            explanation: "The expression involves division by zero or raising zero to a negative power, which is undefined.".into(),
            suggestion: Some("Check your denominator or exponent for cases where the value might be zero.".into()),
            example: None,
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        });
    }
    None
}

fn check_syntax_error(raw: &str) -> Option<ErrorInfo> {
    if raw.contains("incorrect syntax:") {
        let explanation = if raw.contains("is not a prefix operator") {
            "An operator was used in a position where it's not valid. Check for missing operands or misplaced operators.".into()
        } else if raw.contains("is not an infix operator") {
            "Two values appear next to each other without an operator between them.".into()
        } else {
            "The expression contains a syntax error. Check for missing operators, unmatched parentheses, or misplaced characters.".into()
        };

        return Some(ErrorInfo {
            title: "Syntax Error".into(),
            explanation,
            suggestion: Some("Check for missing *, +, or other operators. Ensure all parentheses are matched.".into()),
            example: None,
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        });
    }
    None
}

fn check_arg_count(raw: &str, catalog: &Catalog) -> Option<ErrorInfo> {
    let re = Regex::new(r"Too (few|many) arguments supplied to (\w+)").ok()?;
    let caps = re.captures(raw)?;

    let direction = caps.get(1)?.as_str();
    let func_name = caps.get(2)?.as_str();

    let correct_signatures = catalog
        .get(func_name)
        .map(|f| f.signatures.clone())
        .unwrap_or_default();

    let explanation = format!(
        "Too {} arguments were passed to {}.",
        direction, func_name
    );

    let suggestion = if correct_signatures.is_empty() {
        Some(format!("Check the documentation for {} with describe({}).", func_name, func_name))
    } else {
        Some(format!("Correct usage: {}", correct_signatures.join(" or ")))
    };

    Some(ErrorInfo {
        title: "Wrong Argument Count".into(),
        explanation,
        suggestion,
        example: None,
        did_you_mean: Vec::new(),
        correct_signatures,
    })
}

fn check_undefined_variable(raw: &str) -> Option<ErrorInfo> {
    let re = Regex::new(r"(?:unbound|undefined) variable (\w+)").ok()?;
    let caps = re.captures(raw)?;
    let var_name = caps.get(1)?.as_str();

    Some(ErrorInfo {
        title: "Undefined Variable".into(),
        explanation: format!("The variable '{}' has not been assigned a value.", var_name),
        suggestion: Some(format!(
            "Assign it first with {}: value; or use it as a symbolic variable.",
            var_name
        )),
        example: Some(format!("{}: 42;", var_name)),
        did_you_mean: Vec::new(),
        correct_signatures: Vec::new(),
    })
}

fn check_undefined_function(raw: &str, catalog: &Catalog) -> Option<ErrorInfo> {
    // Maxima says things like "funcall: no such function: intgrate" or
    // "The function intgrate is not known to Maxima"
    let patterns = [
        Regex::new(r"no such function:?\s+(\w+)").ok()?,
        Regex::new(r"The function (\w+) is not known").ok()?,
    ];

    for re in &patterns {
        if let Some(caps) = re.captures(raw) {
            let func_name = caps.get(1)?.as_str();
            let similar = catalog.find_similar(func_name, 3);

            return Some(ErrorInfo {
                title: "Unknown Function".into(),
                explanation: format!("'{}' is not a known Maxima function.", func_name),
                suggestion: if similar.is_empty() {
                    Some(format!("Check the spelling or use describe(\"{}\") to search.", func_name))
                } else {
                    None
                },
                example: None,
                did_you_mean: similar,
                correct_signatures: Vec::new(),
            });
        }
    }
    None
}

fn check_lisp_error(raw: &str) -> Option<ErrorInfo> {
    if raw.contains("Maxima encountered a Lisp error") || raw.contains("MACSYMA restart") {
        return Some(ErrorInfo {
            title: "Internal Error".into(),
            explanation: "Maxima encountered an internal error in the underlying Lisp system.".into(),
            suggestion: Some("Try restarting the Maxima session. If the error persists, the expression may be too complex or trigger a bug in Maxima.".into()),
            example: None,
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        });
    }
    None
}

fn check_divergent_integral(raw: &str) -> Option<ErrorInfo> {
    if raw.contains("integral is divergent") {
        return Some(ErrorInfo {
            title: "Divergent Integral".into(),
            explanation: "The definite integral does not converge to a finite value.".into(),
            suggestion: Some("Check the integration bounds and integrand. The function may have singularities or not decay fast enough.".into()),
            example: None,
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        });
    }
    None
}

fn check_inconsistent_equations(raw: &str) -> Option<ErrorInfo> {
    if raw.contains("inconsistent equations") {
        return Some(ErrorInfo {
            title: "Inconsistent Equations".into(),
            explanation: "The system of equations has no solution -- the equations contradict each other.".into(),
            suggestion: Some("Check that the equations are correct and compatible.".into()),
            example: None,
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        });
    }
    None
}

fn check_missing_assumption(raw: &str) -> Option<ErrorInfo> {
    let re = Regex::new(r"Is\s+(.+?)\s+(positive|negative|zero|an integer|even|odd)").ok()?;
    if re.is_match(raw) {
        return Some(ErrorInfo {
            title: "Assumption Required".into(),
            explanation: "Maxima needs to know a property of a variable to proceed with the computation.".into(),
            suggestion: Some("Use assume() to declare properties, e.g.: assume(x > 0);".into()),
            example: Some("assume(x > 0);".into()),
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        });
    }
    None
}

fn check_matrix_dimension(raw: &str) -> Option<ErrorInfo> {
    if raw.contains("all rows must be the same length")
        || raw.contains("incompatible dimensions")
    {
        return Some(ErrorInfo {
            title: "Matrix Dimension Error".into(),
            explanation: "Matrix rows have different lengths, or the matrices have incompatible dimensions for the operation.".into(),
            suggestion: Some("Ensure all rows have the same number of elements and matrices have compatible dimensions.".into()),
            example: Some("matrix([1,2,3], [4,5,6]);".into()),
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        });
    }
    None
}

fn check_premature_termination(raw: &str) -> Option<ErrorInfo> {
    if raw.contains("Premature termination") {
        return Some(ErrorInfo {
            title: "Incomplete Expression".into(),
            explanation: "The expression ended unexpectedly. This usually means a missing closing parenthesis, bracket, or semicolon.".into(),
            suggestion: Some("Check that all parentheses () and brackets [] are matched, and the expression ends with ; or $".into()),
            example: None,
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        });
    }
    None
}

fn check_load_failed(raw: &str) -> Option<ErrorInfo> {
    if raw.contains("loadfile: failed to load") || raw.contains("Cannot find file") {
        let re = Regex::new(r"(?:failed to load|Cannot find file)\s+(\S+)").ok();
        let pkg = re
            .and_then(|r| r.captures(raw))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        return Some(ErrorInfo {
            title: "Package Not Found".into(),
            explanation: format!(
                "The package{} could not be found or loaded.",
                pkg.as_ref().map(|p| format!(" '{}'", p)).unwrap_or_default()
            ),
            suggestion: Some("Check the package name. Common packages: draw, simplex, lsquares, descriptive, distrib.".into()),
            example: Some("load(draw);".into()),
            did_you_mean: Vec::new(),
            correct_signatures: Vec::new(),
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        Catalog::load()
    }

    #[test]
    fn test_division_by_zero() {
        let info = enhance_error(
            "expt: undefined: 0 to a negative exponent.\n -- an error.",
            &catalog(),
        );
        assert!(info.is_some());
        assert_eq!(info.unwrap().title, "Division by Zero");
    }

    #[test]
    fn test_syntax_error() {
        let info = enhance_error(
            "incorrect syntax: ! is not a prefix operator",
            &catalog(),
        );
        assert!(info.is_some());
        assert_eq!(info.unwrap().title, "Syntax Error");
    }

    #[test]
    fn test_too_few_args() {
        let info = enhance_error(
            "Too few arguments supplied to integrate; found: 0\n -- an error.",
            &catalog(),
        );
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.title, "Wrong Argument Count");
        assert!(!info.correct_signatures.is_empty());
    }

    #[test]
    fn test_did_you_mean() {
        let info = enhance_error(
            "funcall: no such function: intgrate",
            &catalog(),
        );
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.title, "Unknown Function");
        assert!(info.did_you_mean.contains(&"integrate".to_string()));
    }

    #[test]
    fn test_divergent() {
        let info = enhance_error(
            "defint: integral is divergent.",
            &catalog(),
        );
        assert!(info.is_some());
        assert_eq!(info.unwrap().title, "Divergent Integral");
    }

    #[test]
    fn test_no_match() {
        let info = enhance_error("some random output", &catalog());
        assert!(info.is_none());
    }
}
