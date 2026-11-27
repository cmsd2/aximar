//! Aximar's explanations of Maxima's errors.
//!
//! Each explanation has an ID, such as `error.division_by_zero`, and its
//! messages have that ID and the field as their context
//! (`error.division_by_zero.title`), so the catalog has one entry per
//! field.  The `// i18n:` note on each title says which Maxima text the
//! explanation covers; `docs/maxima-messages.md` lists them.  Text is in
//! English: the window translates it.

use regex::Regex;
use std::sync::LazyLock;

use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::maxima::types::ErrorInfo;
use crate::msg_ctx;

/// User functions and lambdas print their definition; old Maxima printed
/// built-in functions' names alone
static ARG_COUNT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)Too (few|many) arguments supplied to (.+?)(?:; found: (.*))?$").unwrap()
});

static UNDEFINED_VAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:unbound|undefined) variable (\w+)").unwrap());

static UNDEFINED_FUNC_RE_1: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"no such function:?\s+(\w+)").unwrap());

static UNDEFINED_FUNC_RE_2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"The function (\w+) is not known").unwrap());

static MISSING_ASSUMPTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Is\s+(.+?)\s+(positive|negative|zero|an integer|even|odd)").unwrap()
});

static LOAD_FAILED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:failed to load|Cannot find file)\s+(\S+)").unwrap());

/// The parser's message, after Maxima's "incorrect syntax: "
static SYNTAX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)incorrect syntax: (.+?)\s*$").unwrap());

static SYNTAX_MISSING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Missing (\S+)$").unwrap());

static SYNTAX_OPERATOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+) is not an? (prefix|infix) operator$").unwrap());

pub fn enhance_error(raw_error: &str, catalog: &Catalog) -> Option<ErrorInfo> {
    enhance_error_with_packages(raw_error, catalog, None)
}

pub fn enhance_error_with_packages(
    raw_error: &str,
    catalog: &Catalog,
    packages: Option<&PackageCatalog>,
) -> Option<ErrorInfo> {
    check_division_by_zero(raw_error)
        // Before the other syntax errors: it's one of them
        .or_else(|| check_premature_termination(raw_error))
        .or_else(|| check_syntax_error(raw_error))
        .or_else(|| check_arg_count(raw_error, catalog))
        .or_else(|| check_undefined_variable(raw_error))
        .or_else(|| check_undefined_function(raw_error, catalog, packages))
        .or_else(|| check_lisp_error(raw_error))
        .or_else(|| check_divergent_integral(raw_error))
        .or_else(|| check_inconsistent_equations(raw_error))
        .or_else(|| check_missing_assumption(raw_error))
        .or_else(|| check_matrix_dimension(raw_error))
        .or_else(|| check_load_failed(raw_error))
}

fn check_division_by_zero(raw: &str) -> Option<ErrorInfo> {
    if !(raw.contains("0 to a negative exponent") || raw.contains("Division by 0")) {
        return None;
    }
    Some(
        ErrorInfo::new(
            "error.division_by_zero",
            // i18n: Maxima: "expt: undefined: 0 to a negative exponent." or "Division by 0"
            msg_ctx!("error.division_by_zero.title", "Division by Zero"),
            msg_ctx!(
                "error.division_by_zero.explanation",
                "The expression involves division by zero or raising zero to a negative power, which is undefined."
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.division_by_zero.suggestion",
            "Check your denominator or exponent for cases where the value might be zero."
        )),
    )
}

fn check_premature_termination(raw: &str) -> Option<ErrorInfo> {
    if !raw.contains("Premature termination") {
        return None;
    }
    Some(
        ErrorInfo::new(
            "error.incomplete_input",
            // i18n: Maxima: "incorrect syntax: Premature termination of input at ;."
            msg_ctx!("error.incomplete_input.title", "Incomplete Expression"),
            msg_ctx!(
                "error.incomplete_input.explanation",
                "The expression ended unexpectedly. This usually means a missing closing parenthesis, bracket, or semicolon."
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.incomplete_input.suggestion",
            "Check that all parentheses () and brackets [] are matched, and the expression ends with ; or $"
        )),
    )
}

fn check_syntax_error(raw: &str) -> Option<ErrorInfo> {
    let message = SYNTAX_RE.captures(raw)?.get(1)?.as_str();

    if let Some(caps) = SYNTAX_MISSING_RE.captures(message) {
        let delimiter = &caps[1];
        return Some(
            ErrorInfo::new(
                "error.syntax_missing",
                // i18n: Maxima: "incorrect syntax: Missing )", or another closing delimiter
                msg_ctx!("error.syntax_missing.title", "Syntax Error"),
                msg_ctx!(
                    "error.syntax_missing.explanation",
                    // i18n: delimiter is a closing bracket, such as ) or ]
                    "A closing {delimiter} is missing.",
                    delimiter = delimiter
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.syntax_missing.suggestion",
                "Check that every opening bracket has its closing {delimiter}, in the right order.",
                delimiter = delimiter
            )),
        );
    }

    if let Some(caps) = SYNTAX_OPERATOR_RE.captures(message) {
        let token = &caps[1];
        let explanation = if &caps[2] == "prefix" {
            msg_ctx!(
                "error.syntax_operator.explanation.prefix",
                // i18n: token is what Maxima read where an expression should start, such as !
                "\"{token}\" can't start an expression here. Check for a missing operand or a misplaced operator.",
                token = token
            )
        } else {
            msg_ctx!(
                "error.syntax_operator.explanation.infix",
                // i18n: token is the value Maxima read after another value, as y in "x y"
                "Two values appear next to each other without an operator between them, before \"{token}\".",
                token = token
            )
        };
        return Some(
            ErrorInfo::new(
                "error.syntax_operator",
                // i18n: Maxima: "incorrect syntax: y is not an infix operator" or "! is not a prefix operator"
                msg_ctx!("error.syntax_operator.title", "Syntax Error"),
                explanation,
            )
            .with_suggestion(msg_ctx!(
                "error.syntax_operator.suggestion",
                "Check for missing *, +, or other operators. Ensure all parentheses are matched."
            )),
        );
    }

    Some(
        ErrorInfo::new(
            "error.syntax",
            // i18n: Maxima: any other "incorrect syntax: ..." message
            msg_ctx!("error.syntax.title", "Syntax Error"),
            msg_ctx!(
                "error.syntax.explanation",
                // i18n: message is Maxima's own description of the error, in English
                "The expression contains a syntax error: {message}. Check for missing operators, unmatched parentheses, or misplaced characters.",
                message = message.trim_end_matches('.')
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.syntax.suggestion",
            "Check for missing *, +, or other operators. Ensure all parentheses are matched."
        )),
    )
}

fn check_arg_count(raw: &str, catalog: &Catalog) -> Option<ErrorInfo> {
    let caps = ARG_COUNT_RE.captures(raw)?;
    let too_few = &caps[1] == "few";
    let called = caps[2].trim();

    // i18n: Maxima: "Too many arguments supplied to f(x); found: [1, 2]" or "Too few ..."
    let title = msg_ctx!("error.arg_count.title", "Wrong Argument Count");

    // A user function or lambda: Maxima prints its definition
    if let (Some(paren), Some(found)) = (called.find('('), caps.get(3)) {
        let func = called[..paren].trim();
        return Some(
            ErrorInfo::new(
                "error.arg_count",
                title,
                msg_ctx!(
                    "error.arg_count.explanation",
                    // i18n: func is the function's name; definition is its name and parameters
                    // as defined, such as f(x); found is the list of arguments it was given
                    "{func} is defined as {definition}, but was called with {found}.",
                    func = func,
                    definition = called,
                    found = found.as_str()
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.arg_count.suggestion",
                "Call {func} with the arguments {definition} names, or change its definition.",
                func = func,
                definition = called
            )),
        );
    }

    let func = called;
    let correct_signatures = catalog
        .signature_info(func)
        .map(|sigs| sigs.into_iter().map(|(sig, _, _)| sig).collect::<Vec<_>>())
        .unwrap_or_default();

    let explanation = if too_few {
        msg_ctx!(
            "error.arg_count.explanation.too_few",
            "Too few arguments were passed to {func}.",
            func = func
        )
    } else {
        msg_ctx!(
            "error.arg_count.explanation.too_many",
            "Too many arguments were passed to {func}.",
            func = func
        )
    };
    let suggestion = if correct_signatures.is_empty() {
        msg_ctx!(
            "error.arg_count.suggestion.describe",
            "Check the documentation for {func} with describe({func}).",
            func = func
        )
    } else {
        msg_ctx!(
            "error.arg_count.suggestion.signatures",
            // i18n: signatures is one or more ways to call the function, separated by semicolons
            "Correct usage: {signatures}",
            signatures = correct_signatures.join("; ")
        )
    };
    let mut info =
        ErrorInfo::new("error.arg_count", title, explanation).with_suggestion(suggestion);
    info.correct_signatures = correct_signatures;
    Some(info)
}

fn check_undefined_variable(raw: &str) -> Option<ErrorInfo> {
    let caps = UNDEFINED_VAR_RE.captures(raw)?;
    let name = caps.get(1)?.as_str();
    Some(
        ErrorInfo::new(
            "error.undefined_variable",
            // i18n: Maxima: "unbound variable x" or "undefined variable x"
            msg_ctx!("error.undefined_variable.title", "Undefined Variable"),
            msg_ctx!(
                "error.undefined_variable.explanation",
                "The variable \"{name}\" has not been assigned a value.",
                name = name
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.undefined_variable.suggestion",
            "Assign it first with {name}: value; or use it as a symbolic variable.",
            name = name
        ))
        .with_example(format!("{name}: 42;")),
    )
}

fn check_undefined_function(
    raw: &str,
    catalog: &Catalog,
    packages: Option<&PackageCatalog>,
) -> Option<ErrorInfo> {
    let caps = UNDEFINED_FUNC_RE_1
        .captures(raw)
        .or_else(|| UNDEFINED_FUNC_RE_2.captures(raw))?;
    let func = caps.get(1)?.as_str();
    // i18n: Maxima: "funcall: no such function: f" or "The function f is not known to Maxima"
    let title = msg_ctx!("error.unknown_function.title", "Unknown Function");

    // A function from a package that isn't loaded
    if let Some(package) = packages.and_then(|p| p.package_for_function(func)) {
        return Some(
            ErrorInfo::new(
                "error.unknown_function",
                title,
                msg_ctx!(
                    "error.unknown_function.explanation.in_package",
                    "\"{func}\" is provided by the \"{package}\" package, which needs to be loaded first.",
                    func = func,
                    package = package
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.unknown_function.suggestion.in_package",
                "Add load(\"{package}\")$ before using this function.",
                package = package
            ))
            .with_example(format!("load(\"{package}\")$")),
        );
    }

    let similar = catalog.find_similar(func, 3);
    let mut info = ErrorInfo::new(
        "error.unknown_function",
        title,
        msg_ctx!(
            "error.unknown_function.explanation",
            "\"{func}\" is not a known Maxima function.",
            func = func
        ),
    );
    if similar.is_empty() {
        info.suggestion = Some(msg_ctx!(
            "error.unknown_function.suggestion",
            "Check the spelling or use describe(\"{func}\") to search.",
            func = func
        ));
    }
    info.did_you_mean = similar;
    Some(info)
}

fn check_lisp_error(raw: &str) -> Option<ErrorInfo> {
    if !(raw.contains("Maxima encountered a Lisp error") || raw.contains("MACSYMA restart")) {
        return None;
    }
    Some(
        ErrorInfo::new(
            "error.lisp",
            // i18n: Maxima: "Maxima encountered a Lisp error:" and the condition
            msg_ctx!("error.lisp.title", "Internal Error"),
            msg_ctx!(
                "error.lisp.explanation",
                "Maxima encountered an internal error in the underlying Lisp system."
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.lisp.suggestion",
            "Try restarting the Maxima session. If the error persists, the expression may be too complex or trigger a bug in Maxima."
        )),
    )
}

fn check_divergent_integral(raw: &str) -> Option<ErrorInfo> {
    if !raw.contains("integral is divergent") {
        return None;
    }
    Some(
        ErrorInfo::new(
            "error.divergent_integral",
            // i18n: Maxima: "defint: integral is divergent."
            msg_ctx!("error.divergent_integral.title", "Divergent Integral"),
            msg_ctx!(
                "error.divergent_integral.explanation",
                "The definite integral does not converge to a finite value."
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.divergent_integral.suggestion",
            "Check the integration bounds and integrand. The function may have singularities or not decay fast enough."
        )),
    )
}

fn check_inconsistent_equations(raw: &str) -> Option<ErrorInfo> {
    if !raw.contains("inconsistent equations") {
        return None;
    }
    Some(
        ErrorInfo::new(
            "error.inconsistent_equations",
            msg_ctx!(
                "error.inconsistent_equations.title",
                // i18n: Maxima: "solve: inconsistent equations" and similar
                "Inconsistent Equations"
            ),
            msg_ctx!(
                "error.inconsistent_equations.explanation",
                "The system of equations has no solution: the equations contradict each other."
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.inconsistent_equations.suggestion",
            "Check that the equations are correct and compatible."
        )),
    )
}

fn check_missing_assumption(raw: &str) -> Option<ErrorInfo> {
    if !MISSING_ASSUMPTION_RE.is_match(raw) {
        return None;
    }
    Some(
        ErrorInfo::new(
            "question.other",
            // i18n: Maxima asked a question, such as "Is x positive, negative or zero?"
            msg_ctx!("question.other.title", "Assumption Required"),
            msg_ctx!(
                "question.other.explanation",
                "Maxima needs to know a property of a variable to proceed with the computation."
            ),
        )
        .with_suggestion(msg_ctx!(
            "question.other.suggestion",
            "Use assume() to declare properties, e.g.: assume(x > 0);"
        ))
        .with_example("assume(x > 0);"),
    )
}

fn check_matrix_dimension(raw: &str) -> Option<ErrorInfo> {
    if !(raw.contains("all rows must be the same length")
        || raw.contains("incompatible dimensions"))
    {
        return None;
    }
    Some(
        ErrorInfo::new(
            "error.matrix_dimension",
            // i18n: Maxima: "matrix: all rows must be the same length." or "incompatible dimensions"
            msg_ctx!("error.matrix_dimension.title", "Matrix Dimension Error"),
            msg_ctx!(
                "error.matrix_dimension.explanation",
                "Matrix rows have different lengths, or the matrices have incompatible dimensions for the operation."
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.matrix_dimension.suggestion",
            "Ensure all rows have the same number of elements and matrices have compatible dimensions."
        ))
        .with_example("matrix([1,2,3], [4,5,6]);"),
    )
}

fn check_load_failed(raw: &str) -> Option<ErrorInfo> {
    if !(raw.contains("loadfile: failed to load") || raw.contains("Cannot find file")) {
        return None;
    }
    let package = LOAD_FAILED_RE
        .captures(raw)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str());
    let explanation = match package {
        Some(package) => msg_ctx!(
            "error.package_not_found.explanation",
            "The package \"{package}\" could not be found or loaded.",
            package = package
        ),
        None => msg_ctx!(
            "error.package_not_found.explanation.unnamed",
            "The package could not be found or loaded."
        ),
    };
    Some(
        ErrorInfo::new(
            "error.package_not_found",
            // i18n: Maxima: "loadfile: failed to load x" or "Cannot find file x"
            msg_ctx!("error.package_not_found.title", "Package Not Found"),
            explanation,
        )
        .with_suggestion(msg_ctx!(
            "error.package_not_found.suggestion",
            "Check the package name. Common packages: draw, simplex, lsquares, descriptive, distrib."
        ))
        .with_example("load(draw);"),
    )
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
        assert_eq!(info.unwrap().title.text, "Division by Zero");
    }

    #[test]
    fn test_syntax_error() {
        let info = enhance_error("incorrect syntax: ! is not a prefix operator", &catalog());
        assert!(info.is_some());
        assert_eq!(info.unwrap().title.text, "Syntax Error");
    }

    #[test]
    fn test_too_few_args() {
        let info = enhance_error(
            "Too few arguments supplied to integrate; found: 0\n -- an error.",
            &catalog(),
        );
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.title.text, "Wrong Argument Count");
        assert!(!info.correct_signatures.is_empty());
    }

    #[test]
    fn test_did_you_mean() {
        let info = enhance_error("funcall: no such function: intgrate", &catalog());
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.title.text, "Unknown Function");
        assert!(info.did_you_mean.contains(&"integrate".to_string()));
    }

    #[test]
    fn test_divergent() {
        let info = enhance_error("defint: integral is divergent.", &catalog());
        assert!(info.is_some());
        assert_eq!(info.unwrap().title.text, "Divergent Integral");
    }

    #[test]
    fn test_no_match() {
        let info = enhance_error("some random output", &catalog());
        assert!(info.is_none());
    }

    #[test]
    fn test_division_by_zero_alternate() {
        let info = enhance_error("Division by 0", &catalog());
        assert!(info.is_some());
        assert_eq!(info.unwrap().title.text, "Division by Zero");
    }

    #[test]
    fn test_syntax_error_infix() {
        let info = enhance_error("incorrect syntax: x is not an infix operator", &catalog());
        let info = info.unwrap();
        assert_eq!(info.title.text, "Syntax Error");
        assert!(info.explanation.text.contains("Two values"));
    }

    #[test]
    fn test_syntax_error_generic() {
        let info = enhance_error("incorrect syntax: unexpected end", &catalog());
        let info = info.unwrap();
        assert_eq!(info.title.text, "Syntax Error");
        assert!(info.explanation.text.contains("syntax error"));
    }

    #[test]
    fn test_too_many_args() {
        let info = enhance_error("Too many arguments supplied to sin", &catalog());
        assert!(info.is_some());
        assert_eq!(info.unwrap().title.text, "Wrong Argument Count");
    }

    #[test]
    fn test_arg_count_unknown_func() {
        let info = enhance_error("Too few arguments supplied to myfunc", &catalog());
        let info = info.unwrap();
        assert_eq!(info.title.text, "Wrong Argument Count");
        assert!(info.correct_signatures.is_empty());
        assert!(info.suggestion.as_ref().unwrap().text.contains("describe"));
    }

    #[test]
    fn test_undefined_variable() {
        let info = enhance_error("unbound variable foo", &catalog());
        let info = info.unwrap();
        assert_eq!(info.title.text, "Undefined Variable");
        assert!(info.explanation.text.contains("foo"));
    }

    #[test]
    fn test_undefined_func_not_known() {
        let info = enhance_error("The function intgrate is not known to Maxima", &catalog());
        let info = info.unwrap();
        assert_eq!(info.title.text, "Unknown Function");
    }

    #[test]
    fn test_lisp_error() {
        let info = enhance_error("Maxima encountered a Lisp error", &catalog());
        assert_eq!(info.unwrap().title.text, "Internal Error");
    }

    #[test]
    fn test_macsyma_restart() {
        let info = enhance_error("MACSYMA restart", &catalog());
        assert_eq!(info.unwrap().title.text, "Internal Error");
    }

    #[test]
    fn test_inconsistent_equations() {
        let info = enhance_error("solve: inconsistent equations", &catalog());
        assert_eq!(info.unwrap().title.text, "Inconsistent Equations");
    }

    #[test]
    fn test_missing_assumption() {
        let info = enhance_error("Is x positive, negative, or zero?", &catalog());
        assert_eq!(info.unwrap().title.text, "Assumption Required");
    }

    #[test]
    fn test_missing_assumption_integer() {
        let info = enhance_error("Is n an integer?", &catalog());
        assert_eq!(info.unwrap().title.text, "Assumption Required");
    }

    #[test]
    fn test_matrix_dimension_rows() {
        let info = enhance_error("all rows must be the same length", &catalog());
        assert_eq!(info.unwrap().title.text, "Matrix Dimension Error");
    }

    #[test]
    fn test_matrix_dimension_incompatible() {
        let info = enhance_error("incompatible dimensions", &catalog());
        assert_eq!(info.unwrap().title.text, "Matrix Dimension Error");
    }

    #[test]
    fn test_premature_termination() {
        let info = enhance_error("Premature termination", &catalog());
        assert_eq!(info.unwrap().title.text, "Incomplete Expression");
    }

    #[test]
    fn test_load_failed() {
        let info = enhance_error("loadfile: failed to load mypackage", &catalog());
        let info = info.unwrap();
        assert_eq!(info.title.text, "Package Not Found");
        assert!(info.explanation.text.contains("mypackage"));
    }

    #[test]
    fn test_cannot_find_file() {
        let info = enhance_error("Cannot find file draw", &catalog());
        let info = info.unwrap();
        assert_eq!(info.title.text, "Package Not Found");
        assert!(info.explanation.text.contains("draw"));
    }

    #[test]
    fn test_unknown_function_suggests_package() {
        use crate::catalog::packages::PackageCatalog;
        let cat = catalog();
        let pkgs = PackageCatalog::load();
        let info =
            enhance_error_with_packages("funcall: no such function: pdf_normal", &cat, Some(&pkgs));
        let info = info.unwrap();
        assert_eq!(info.title.text, "Unknown Function");
        assert!(
            info.explanation.text.contains("distrib"),
            "Expected 'distrib' in explanation: {}",
            info.explanation.text
        );
        assert!(info.example.is_some());
        assert!(info.example.unwrap().contains("load(\"distrib\")"));
    }

    #[test]
    fn test_unknown_function_no_package_fallback() {
        use crate::catalog::packages::PackageCatalog;
        let cat = catalog();
        let pkgs = PackageCatalog::load();
        // intgrate is a typo, not a package function
        let info =
            enhance_error_with_packages("funcall: no such function: intgrate", &cat, Some(&pkgs));
        let info = info.unwrap();
        assert_eq!(info.title.text, "Unknown Function");
        assert!(info.did_you_mean.contains(&"integrate".to_string()));
    }

    fn ids(raw: &str) -> (String, Vec<String>) {
        let info = enhance_error(raw, &catalog()).expect("an explanation");
        let ids = info.messages().map(|(_, m)| m.id.clone()).collect();
        (info.id, ids)
    }

    #[test]
    fn explanations_carry_their_id() {
        let (id, messages) = ids("expt: undefined: 0 to a negative exponent.");
        assert_eq!(id, "error.division_by_zero");
        assert_eq!(
            messages,
            [
                "error.division_by_zero.title",
                "error.division_by_zero.explanation",
                "error.division_by_zero.suggestion"
            ]
        );
    }

    #[test]
    fn parameters_fill_the_text() {
        let info = enhance_error(
            "Too many arguments supplied to f(x); found: [1, 2]\n -- an error.",
            &catalog(),
        )
        .unwrap();
        assert_eq!(info.id, "error.arg_count");
        assert_eq!(info.title.text, "Wrong Argument Count");
        let e = &info.explanation;
        assert_eq!(e.params["func"], "f");
        assert_eq!(e.params["definition"], "f(x)");
        assert_eq!(e.params["found"], "[1, 2]");
        assert_eq!(e.text, "f is defined as f(x), but was called with [1, 2].");
        // A user function isn't in the manual
        let suggestion = &info.suggestion.unwrap().text;
        assert!(!suggestion.contains("describe"), "{suggestion}");
        assert!(info.correct_signatures.is_empty());
    }

    #[test]
    fn a_lambda_is_named_by_its_definition() {
        let info = enhance_error(
            "Too many arguments supplied to lambda([x], x); found: [1, 2]",
            &catalog(),
        )
        .unwrap();
        assert_eq!(info.explanation.params["definition"], "lambda([x], x)");
    }

    #[test]
    fn syntax_errors_have_their_own_ids() {
        let missing = enhance_error("incorrect syntax: Missing )\nf(x;\n  ^", &catalog()).unwrap();
        assert_eq!(missing.id, "error.syntax_missing");
        assert_eq!(missing.explanation.params["delimiter"], ")");

        let (id, _) = ids("incorrect syntax: Premature termination of input at ;.\n1+;\n ^");
        assert_eq!(id, "error.incomplete_input");

        let infix = enhance_error(
            "incorrect syntax: y is not an infix operator\nx y;",
            &catalog(),
        )
        .unwrap();
        assert_eq!(infix.id, "error.syntax_operator");
        assert_eq!(infix.explanation.params["token"], "y");

        let other = enhance_error("incorrect syntax: Too many )'s\n1+2)", &catalog()).unwrap();
        assert_eq!(other.id, "error.syntax");
        assert_eq!(other.explanation.params["message"], "Too many )'s");
    }

    #[test]
    fn json_keeps_the_english_text_and_adds_the_sources() {
        let info = enhance_error("unbound variable foo", &catalog()).unwrap();
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["id"], "error.undefined_variable");
        assert_eq!(json["title"], "Undefined Variable");
        assert_eq!(
            json["explanation"],
            "The variable \"foo\" has not been assigned a value."
        );
        assert_eq!(json["params"]["name"], "foo");
        assert_eq!(
            json["messages"]["explanation"]["id"],
            "error.undefined_variable.explanation"
        );
        assert_eq!(
            json["messages"]["explanation"]["template"],
            "The variable \"{name}\" has not been assigned a value."
        );
        assert_eq!(json["example"], "foo: 42;");

        let back: ErrorInfo = serde_json::from_value(json).unwrap();
        assert_eq!(back, info);
    }

    #[test]
    fn json_from_before_ids_still_reads() {
        let json = serde_json::json!({
            "title": "Division by Zero",
            "explanation": "x",
            "suggestion": null,
            "example": null,
            "did_you_mean": [],
            "correct_signatures": [],
        });
        let info: ErrorInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.id, "");
        assert_eq!(info.title.text, "Division by Zero");
        assert_eq!(info.title.template, "Division by Zero");
    }
}
