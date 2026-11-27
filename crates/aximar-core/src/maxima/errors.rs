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
use crate::error::AppError;
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

/// A question Maxima asked, as `ax_questions.lisp` turns it into an error
static QUESTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)Maxima needs to know: (.+?)\s*$").unwrap());

/// The sign questions, by the answers they take
static SIGN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^Is (.+?) (positive, negative or zero|positive or zero|negative or zero|positive or negative|zero or nonzero)\?$").unwrap()
});

static EQUAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Is (.+) equal to (.+)\?$").unwrap());

/// `ask-prop`: "Is n an integer?", and for askinteger's even and odd,
/// "Is n an even number?"
static PROPERTY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Is (.+?) an? ([a-z]\w*)(?: number)?\?$").unwrap());

static LOAD_FAILED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:failed to load|Cannot find file)\s+(\S+)").unwrap());

static LOG_ZERO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^log: encountered log\(0\)\.").unwrap());

/// "part: fell off the end.", and the same from inpart, rest and others
static PART_INDEX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^(\w+): fell off the end\.").unwrap());

static PART_ATOM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(\w+): argument must be a non-atomic expression; found (.+?)\s*$").unwrap()
});

static ASSIGN_TARGET_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^assignment: cannot assign to (.+?)\s*$").unwrap());

/// "first: empty argument.", and the same from last and others
static EMPTY_LIST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^(\w+): empty argument\.").unwrap());

static NOT_POLYNOMIAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^(\w+): expected a polynomial; found (.+?)\s*$").unwrap());

/// A Lisp error, with its condition on one line after the colon
static LISP_ERROR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)Maxima encountered a Lisp error:?(.*)$").unwrap());

/// SBCL's type error: "The value 1 is not of type LIST"
static LISP_TYPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"The value (.+?) is not of type (\S+)").unwrap());

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
    check_question(raw_error)
        .or_else(|| check_lisp_error(raw_error))
        .or_else(|| check_division_by_zero(raw_error))
        // Before the other syntax errors: it's one of them
        .or_else(|| check_premature_termination(raw_error))
        .or_else(|| check_syntax_error(raw_error))
        .or_else(|| check_arg_count(raw_error, catalog))
        .or_else(|| check_undefined_variable(raw_error))
        .or_else(|| check_undefined_function(raw_error, catalog, packages))
        .or_else(|| check_survey_errors(raw_error))
        .or_else(|| check_divergent_integral(raw_error))
        .or_else(|| check_inconsistent_equations(raw_error))
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
            "Check that all parentheses `()` and brackets `[]` are matched, and the expression ends with `;` or `$`"
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
                    "A closing `{delimiter}` is missing.",
                    delimiter = delimiter
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.syntax_missing.suggestion",
                "Check that every opening bracket has its closing `{delimiter}`, in the right order.",
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
                "`{token}` can't start an expression here. Check for a missing operand or a misplaced operator.",
                token = token
            )
        } else {
            msg_ctx!(
                "error.syntax_operator.explanation.infix",
                // i18n: token is the value Maxima read after another value, as y in "x y"
                "Two values appear next to each other without an operator between them, before `{token}`.",
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
                "Check for missing `*`, `+`, or other operators. Ensure all parentheses are matched."
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
            "Check for missing `*`, `+`, or other operators. Ensure all parentheses are matched."
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
                    "`{func}` is defined as `{definition}`, but was called with `{found}`.",
                    func = func,
                    definition = called,
                    found = found.as_str()
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.arg_count.suggestion",
                "Call `{func}` with the arguments `{definition}` names, or change its definition.",
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
            "Too few arguments were passed to `{func}`.",
            func = func
        )
    } else {
        msg_ctx!(
            "error.arg_count.explanation.too_many",
            "Too many arguments were passed to `{func}`.",
            func = func
        )
    };
    let suggestion = if correct_signatures.is_empty() {
        msg_ctx!(
            "error.arg_count.suggestion.describe",
            "Check the documentation for `{func}` with `describe({func})`.",
            func = func
        )
    } else {
        msg_ctx!(
            "error.arg_count.suggestion.signatures",
            // i18n: signatures is one or more ways to call the function, separated by semicolons
            "Correct usage: `{signatures}`",
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
                "The variable `{name}` has not been assigned a value.",
                name = name
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.undefined_variable.suggestion",
            "Assign it first with `{name}: value;` or use it as a symbolic variable.",
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
                    "`{func}` is provided by the `{package}` package, which needs to be loaded first.",
                    func = func,
                    package = package
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.unknown_function.suggestion.in_package",
                "Add `load(\"{package}\")$` before using this function.",
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
            "`{func}` is not a known Maxima function.",
            func = func
        ),
    );
    if similar.is_empty() {
        info.suggestion = Some(msg_ctx!(
            "error.unknown_function.suggestion",
            "Check the spelling or use `describe(\"{func}\")` to search.",
            func = func
        ));
    }
    info.did_you_mean = similar;
    Some(info)
}

fn check_lisp_error(raw: &str) -> Option<ErrorInfo> {
    let condition = if let Some(caps) = LISP_ERROR_RE.captures(raw) {
        caps.get(1).map_or("", |m| m.as_str()).trim()
    } else if raw.contains("MACSYMA restart") {
        raw.trim()
    } else {
        return None;
    };

    if let Some(caps) = LISP_TYPE_RE.captures(condition) {
        let value = caps[1].trim();
        let lisp_type = caps[2].trim().to_lowercase();
        return Some(
            ErrorInfo::new(
                "error.lisp_type",
                // i18n: Maxima: "Maxima encountered a Lisp error: The value 1 is not of type LIST"
                msg_ctx!("error.lisp_type.title", "Wrong Kind of Value"),
                msg_ctx!(
                    "error.lisp_type.explanation",
                    // i18n: value is the value given, such as 1; lisp_type is a Lisp
                    // type name, such as list or number, which stays in English
                    "A function inside Maxima expected a value of type `{lisp_type}` but got `{value}`.",
                    value = value,
                    lisp_type = lisp_type
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.lisp_type.suggestion",
                "Check the arguments of the functions in this cell, especially Lisp functions called with `?`, which don't check their arguments."
            )),
        );
    }

    if condition.contains("Control stack exhausted") {
        return Some(
            ErrorInfo::new(
                "error.recursion",
                // i18n: Maxima: "Maxima encountered a Lisp error: Control stack exhausted ..."
                msg_ctx!("error.recursion.title", "Runaway Recursion"),
                msg_ctx!(
                    "error.recursion.explanation",
                    "A function called itself, directly or through others, until Maxima ran out of room for the calls."
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.recursion.suggestion",
                "Check that each recursive function has a case that stops it, such as `f(0) := 1`."
            )),
        );
    }

    if condition.contains("FLOATING-POINT-OVERFLOW") {
        return Some(
            ErrorInfo::new(
                "error.float_overflow",
                // i18n: Maxima: "Maxima encountered a Lisp error: arithmetic error FLOATING-POINT-OVERFLOW signalled"
                msg_ctx!("error.float_overflow.title", "Number Too Large"),
                msg_ctx!(
                    "error.float_overflow.explanation",
                    "A floating-point result was too large to represent."
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.float_overflow.suggestion",
                "Keep the computation exact, or use `bfloat()` for very large numbers."
            )),
        );
    }

    Some(
        ErrorInfo::new(
            "error.lisp",
            // i18n: Maxima: "Maxima encountered a Lisp error:" and any other condition
            msg_ctx!("error.lisp.title", "Internal Error"),
            msg_ctx!(
                "error.lisp.explanation",
                // i18n: condition is the Lisp system's description of the error, in English
                "Maxima encountered an internal error in the underlying Lisp system: {condition}",
                condition = condition
            ),
        )
        .with_suggestion(msg_ctx!(
            "error.lisp.suggestion",
            "Try restarting the Maxima session. If the error persists, the expression may be too complex or trigger a bug in Maxima."
        )),
    )
}

/// Errors common in coursework: the survey in the messages-and-localisation
/// design
fn check_survey_errors(raw: &str) -> Option<ErrorInfo> {
    if LOG_ZERO_RE.is_match(raw) {
        return Some(
            ErrorInfo::new(
                "error.log_zero",
                // i18n: Maxima: "log: encountered log(0)."
                msg_ctx!("error.log_zero.title", "Logarithm of Zero"),
                msg_ctx!(
                    "error.log_zero.explanation",
                    "The expression takes the logarithm of 0, which is undefined."
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.log_zero.suggestion",
                "Check the argument of `log` for values that make it 0, or take a limit instead."
            )),
        );
    }

    if let Some(caps) = PART_INDEX_RE.captures(raw) {
        let func = &caps[1];
        return Some(
            ErrorInfo::new(
                "error.part_index",
                // i18n: Maxima: "part: fell off the end.", or the same from inpart or rest
                msg_ctx!("error.part_index.title", "Index Out of Range"),
                msg_ctx!(
                    "error.part_index.explanation",
                    // i18n: func is the Maxima function, such as part or inpart
                    "`{func}` asked for a part beyond the end of the expression or list.",
                    func = func
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.part_index.suggestion",
                "Check the index against `length()` of the expression or list."
            )),
        );
    }

    if let Some(caps) = PART_ATOM_RE.captures(raw) {
        let (func, found) = (&caps[1], &caps[2]);
        return Some(
            ErrorInfo::new(
                "error.part_atom",
                // i18n: Maxima: "part: argument must be a non-atomic expression; found x"
                msg_ctx!("error.part_atom.title", "No Parts to Take"),
                msg_ctx!(
                    "error.part_atom.explanation",
                    // i18n: func is the Maxima function, such as part; found is what it was given, such as x
                    "`{func}` needs an expression with parts, such as a list or a sum, but got `{found}`.",
                    func = func,
                    found = found
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.part_atom.suggestion",
                "Check that the variable holds the list or expression you meant."
            )),
        );
    }

    if let Some(caps) = ASSIGN_TARGET_RE.captures(raw) {
        let target = &caps[1];
        return Some(
            ErrorInfo::new(
                "error.assign_target",
                // i18n: Maxima: "assignment: cannot assign to 2"
                msg_ctx!("error.assign_target.title", "Can't Assign"),
                msg_ctx!(
                    "error.assign_target.explanation",
                    // i18n: target is what the cell tried to assign to, such as 2 or %pi
                    "The left side of `:` must be a variable name, but it is `{target}`.",
                    target = target
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.assign_target.suggestion",
                "To write an equation, use `=` instead of `:`, as in `x^2 = 4`."
            )),
        );
    }

    if let Some(caps) = EMPTY_LIST_RE.captures(raw) {
        let func = &caps[1];
        return Some(
            ErrorInfo::new(
                "error.empty_list",
                // i18n: Maxima: "first: empty argument.", or the same from last and others
                msg_ctx!("error.empty_list.title", "Empty List"),
                msg_ctx!(
                    "error.empty_list.explanation",
                    // i18n: func is the Maxima function, such as first or last
                    "`{func}` was given an empty list, which has no elements to take.",
                    func = func
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.empty_list.suggestion",
                "Check where the list comes from, for example a `solve()` that found no solutions."
            )),
        );
    }

    if let Some(caps) = NOT_POLYNOMIAL_RE.captures(raw) {
        let (func, found) = (&caps[1], &caps[2]);
        return Some(
            ErrorInfo::new(
                "error.not_polynomial",
                // i18n: Maxima: "allroots: expected a polynomial; found sin(x)"
                msg_ctx!("error.not_polynomial.title", "Not a Polynomial"),
                msg_ctx!(
                    "error.not_polynomial.explanation",
                    // i18n: func is the Maxima function, such as allroots; found is the expression given
                    "`{func}` works only on polynomials, but got `{found}`.",
                    func = func,
                    found = found
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.not_polynomial.suggestion",
                "For other equations, try `find_root()` over an interval, or `solve()`."
            )),
        );
    }

    if raw.contains("algsys: Couldn't reduce system to a polynomial in one variable") {
        return Some(
            ErrorInfo::new(
                "error.algsys_failed",
                // i18n: Maxima: "algsys: Couldn't reduce system to a polynomial in one variable."
                msg_ctx!("error.algsys_failed.title", "Can't Solve the System"),
                msg_ctx!(
                    "error.algsys_failed.explanation",
                    "`algsys`, which `solve` uses for systems, couldn't turn the equations into a polynomial in one variable."
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.algsys_failed.suggestion",
                "For a numerical answer, try `mnewton()` from `load(mnewton)` with a starting point."
            )),
        );
    }

    None
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

/// The explanation for an evaluation that didn't finish: it timed out, or
/// the user cancelled it.  `None` for other failures, which leave no output.
pub fn explain_failure(error: &AppError) -> Option<ErrorInfo> {
    match error {
        AppError::Timeout(seconds) => Some(
            ErrorInfo::new(
                "error.timeout",
                // i18n: Aximar stopped a cell that ran longer than the evaluation timeout
                msg_ctx!("error.timeout.title", "Timed Out"),
                msg_ctx!(
                    "error.timeout.explanation",
                    // i18n: seconds is the timeout from Settings, such as 30
                    "The cell ran for longer than {seconds} seconds, so Aximar stopped it.",
                    seconds = seconds
                ),
            )
            .with_suggestion(msg_ctx!(
                "error.timeout.suggestion",
                "Raise the evaluation timeout in Settings, or try a smaller or numeric version of the computation."
            )),
        ),
        AppError::EvalCancelled(_) => Some(ErrorInfo::new(
            "error.cancelled",
            // i18n: the user stopped a running cell
            msg_ctx!("error.cancelled.title", "Cancelled"),
            msg_ctx!(
                "error.cancelled.explanation",
                "The cell was stopped before it finished."
            ),
        )),
        _ => None,
    }
}

fn check_question(raw: &str) -> Option<ErrorInfo> {
    let question = QUESTION_RE.captures(raw)?.get(1)?.as_str();

    if let Some(caps) = SIGN_RE.captures(question) {
        let expr = &caps[1];
        return Some(match &caps[2] {
            "positive or zero" => {
                let line = format!("assume({expr} > 0)$");
                ErrorInfo::new(
                    "question.sign_pz",
                    // i18n: Maxima: "Maxima needs to know: Is a positive or zero?"
                    msg_ctx!("question.sign_pz.title", "Maxima Needs to Know"),
                    msg_ctx!(
                        "question.sign_pz.explanation",
                        // i18n: expr is a Maxima expression, such as a or b*c-1
                        "Maxima needs to know whether `{expr}` is positive or zero to go on.",
                        expr = expr
                    ),
                )
                .with_suggestion(msg_ctx!(
                    "question.sign_pz.suggestion",
                    // i18n: line is Maxima code that answers the question, such as assume(a > 0)$
                    "Add `{line}` before this line, or in a cell above, and run it again.",
                    line = line.as_str()
                ))
                .with_example(line)
            }
            "negative or zero" => {
                let line = format!("assume({expr} < 0)$");
                ErrorInfo::new(
                    "question.sign_nz",
                    // i18n: Maxima: "Maxima needs to know: Is a negative or zero?"
                    msg_ctx!("question.sign_nz.title", "Maxima Needs to Know"),
                    msg_ctx!(
                        "question.sign_nz.explanation",
                        // i18n: expr is a Maxima expression, such as a or b*c-1
                        "Maxima needs to know whether `{expr}` is negative or zero to go on.",
                        expr = expr
                    ),
                )
                .with_suggestion(msg_ctx!(
                    "question.sign_nz.suggestion",
                    // i18n: line is Maxima code that answers the question, such as assume(a < 0)$
                    "Add `{line}` before this line, or in a cell above, and run it again.",
                    line = line.as_str()
                ))
                .with_example(line)
            }
            "positive or negative" => {
                let line = format!("assume({expr} > 0)$");
                ErrorInfo::new(
                    "question.sign_pn",
                    // i18n: Maxima: "Maxima needs to know: Is k positive or negative?"
                    msg_ctx!("question.sign_pn.title", "Maxima Needs to Know"),
                    msg_ctx!(
                        "question.sign_pn.explanation",
                        // i18n: expr is a Maxima expression, such as k or b*c-1
                        "Maxima needs to know whether `{expr}` is positive or negative to go on.",
                        expr = expr
                    ),
                )
                .with_suggestion(msg_ctx!(
                    "question.sign_pn.suggestion",
                    // i18n: line is Maxima code that answers the question, such as assume(k > 0)$
                    "Add `{line}` before this line, or in a cell above, and run it again.",
                    line = line.as_str()
                ))
                .with_example(line)
            }
            "zero or nonzero" => {
                let line = format!("assume(notequal({expr}, 0))$");
                ErrorInfo::new(
                    "question.zero",
                    // i18n: Maxima: "Maxima needs to know: Is x zero or nonzero?"
                    msg_ctx!("question.zero.title", "Maxima Needs to Know"),
                    msg_ctx!(
                        "question.zero.explanation",
                        // i18n: expr is a Maxima expression, such as x or b*c-1
                        "Maxima needs to know whether `{expr}` is zero to go on.",
                        expr = expr
                    ),
                )
                .with_suggestion(msg_ctx!(
                    "question.zero.suggestion",
                    // i18n: line is Maxima code that answers the question, such as assume(notequal(x, 0))$
                    "Add `{line}` before this line, or in a cell above, and run it again.",
                    line = line.as_str()
                ))
                .with_example(line)
            }
            _ => {
                let line = format!("assume({expr} > 0)$");
                ErrorInfo::new(
                    "question.sign",
                    // i18n: Maxima: "Maxima needs to know: Is a positive, negative or zero?"
                    msg_ctx!("question.sign.title", "Maxima Needs to Know"),
                    msg_ctx!(
                        "question.sign.explanation",
                        // i18n: expr is a Maxima expression, such as a or b*c-1
                        "Maxima needs to know whether `{expr}` is positive, negative or zero to go on.",
                        expr = expr
                    ),
                )
                .with_suggestion(msg_ctx!(
                    "question.sign.suggestion",
                    // i18n: line is Maxima code that answers the question, such as assume(a > 0)$
                    "Add `{line}` before this line, or in a cell above, and run it again.",
                    line = line.as_str()
                ))
                .with_example(line)
            }
        });
    }

    if let Some(caps) = EQUAL_RE.captures(question) {
        let (a, b) = (&caps[1], &caps[2]);
        let line = format!("assume(notequal({a}, {b}))$");
        return Some(
            ErrorInfo::new(
                "question.equal",
                // i18n: Maxima: "Maxima needs to know: Is a equal to -1?"
                msg_ctx!("question.equal.title", "Maxima Needs to Know"),
                msg_ctx!(
                    "question.equal.explanation",
                    // i18n: a and b are Maxima expressions, such as a and -1
                    "Maxima needs to know whether `{a}` equals `{b}` to go on.",
                    a = a,
                    b = b
                ),
            )
            .with_suggestion(msg_ctx!(
                "question.equal.suggestion",
                // i18n: line is Maxima code that answers the question, such as assume(notequal(a, -1))$
                "Add `{line}` before this line, or in a cell above, and run it again.",
                line = line.as_str()
            ))
            .with_example(line),
        );
    }

    if let Some(caps) = PROPERTY_RE.captures(question) {
        let (expr, property) = (&caps[1], &caps[2]);
        let line = format!("declare({expr}, {property})$");
        return Some(
            ErrorInfo::new(
                "question.property",
                // i18n: Maxima: "Maxima needs to know: Is n an integer?", or another property
                msg_ctx!("question.property.title", "Maxima Needs to Know"),
                msg_ctx!(
                    "question.property.explanation",
                    // i18n: expr is a Maxima expression; property is a Maxima
                    // feature name such as integer or even, which stays in English
                    "Maxima needs to know whether `{expr}` has the property `{property}` to go on.",
                    expr = expr,
                    property = property
                ),
            )
            .with_suggestion(msg_ctx!(
                "question.property.suggestion",
                // i18n: line is Maxima code that answers the question, such as declare(n, integer)$
                "Add `{line}` before this line, or in a cell above, and run it again.",
                line = line.as_str()
            ))
            .with_example(line),
        );
    }

    Some(
        ErrorInfo::new(
            "question.other",
            // i18n: Maxima: "Maxima needs to know: ..." with any other question
            msg_ctx!("question.other.title", "Maxima Needs to Know"),
            msg_ctx!(
                "question.other.explanation",
                // i18n: question is Maxima's question, in English
                "Maxima asked \"{question}\" and can't get an answer from a cell.",
                question = question
            ),
        )
        .with_suggestion(msg_ctx!(
            "question.other.suggestion",
            "Use `assume()` or `declare()` before this line to tell Maxima what it needs, then run the cell again."
        )),
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
            "The package `{package}` could not be found or loaded.",
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
            "Check the package name. Common packages: `draw`, `simplex`, `lsquares`, `descriptive`, `distrib`."
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

    fn question(text: &str) -> ErrorInfo {
        let raw = format!(
            "Maxima needs to know: {text}\n -- an error. To debug this try: debugmode(true);"
        );
        enhance_error(&raw, &catalog()).expect("an explanation")
    }

    #[test]
    fn sign_questions_suggest_an_assumption() {
        let cases = [
            (
                "Is a positive, negative or zero?",
                "question.sign",
                "assume(a > 0)$",
            ),
            (
                "Is a positive or zero?",
                "question.sign_pz",
                "assume(a > 0)$",
            ),
            (
                "Is a negative or zero?",
                "question.sign_nz",
                "assume(a < 0)$",
            ),
            (
                "Is k positive or negative?",
                "question.sign_pn",
                "assume(k > 0)$",
            ),
            (
                "Is x zero or nonzero?",
                "question.zero",
                "assume(notequal(x, 0))$",
            ),
            (
                "Is b*c-1 positive, negative or zero?",
                "question.sign",
                "assume(b*c-1 > 0)$",
            ),
            // Real questions: an expression, an absolute value, a noun form
            (
                "Is m+1 positive, negative or zero?",
                "question.sign",
                "assume(m+1 > 0)$",
            ),
            (
                "Is abs(x)-1 positive, negative or zero?",
                "question.sign",
                "assume(abs(x)-1 > 0)$",
            ),
            (
                "Is 'realpart(x) positive, negative or zero?",
                "question.sign",
                "assume('realpart(x) > 0)$",
            ),
        ];
        for (text, id, line) in cases {
            let info = question(text);
            assert_eq!(info.id, id, "{text}");
            assert_eq!(info.title.text, "Maxima Needs to Know");
            assert_eq!(info.example.as_deref(), Some(line), "{text}");
            assert!(info.suggestion.unwrap().text.contains(line), "{text}");
        }
        assert_eq!(
            question("Is a positive or zero?").explanation.params["expr"],
            "a"
        );
    }

    #[test]
    fn an_equality_question_names_both_sides() {
        let info = question("Is a equal to -1?");
        assert_eq!(info.id, "question.equal");
        assert_eq!(info.explanation.params["a"], "a");
        assert_eq!(info.explanation.params["b"], "-1");
        assert_eq!(info.example.as_deref(), Some("assume(notequal(a, -1))$"));
    }

    #[test]
    fn a_property_question_suggests_declare() {
        let info = question("Is n an integer?");
        assert_eq!(info.id, "question.property");
        assert_eq!(info.explanation.params["property"], "integer");
        assert_eq!(info.example.as_deref(), Some("declare(n, integer)$"));
        // askinteger(n, even) and odd add "number"
        let even = question("Is n an even number?");
        assert_eq!(even.explanation.params["expr"], "n");
        assert_eq!(even.explanation.params["property"], "even");
        assert_eq!(even.example.as_deref(), Some("declare(n, even)$"));
        assert_eq!(
            question("Is n an odd number?").example.as_deref(),
            Some("declare(n, odd)$")
        );
    }

    #[test]
    fn other_questions_quote_maxima() {
        let info = question("Answer 1, 2, 3 or 4 :");
        assert_eq!(info.id, "question.other");
        assert_eq!(info.explanation.params["question"], "Answer 1, 2, 3 or 4 :");
        assert!(info.example.is_none());
        // Another wording of a sign question
        assert_eq!(question("Is a positive or not?").id, "question.other");
    }

    #[test]
    fn a_question_without_the_error_is_not_one() {
        assert!(enhance_error("Is x positive, negative or zero?", &catalog()).is_none());
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
        assert_eq!(
            e.text,
            "`f` is defined as `f(x)`, but was called with `[1, 2]`."
        );
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
            "The variable `foo` has not been assigned a value."
        );
        assert_eq!(json["params"]["name"], "foo");
        assert_eq!(
            json["messages"]["explanation"]["id"],
            "error.undefined_variable.explanation"
        );
        assert_eq!(
            json["messages"]["explanation"]["template"],
            "The variable `{name}` has not been assigned a value."
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

    #[test]
    fn survey_errors_have_their_ids() {
        let hint = "\n -- an error. To debug this try: debugmode(true);";
        let cases = [
            ("log: encountered log(0).", "error.log_zero"),
            ("part: fell off the end.", "error.part_index"),
            ("inpart: fell off the end.", "error.part_index"),
            ("rest: fell off the end.", "error.part_index"),
            (
                "part: argument must be a non-atomic expression; found x",
                "error.part_atom",
            ),
            ("assignment: cannot assign to 2", "error.assign_target"),
            ("first: empty argument.", "error.empty_list"),
            ("last: empty argument.", "error.empty_list"),
            (
                "allroots: expected a polynomial; found sin(x)",
                "error.not_polynomial",
            ),
            (
                "algsys: Couldn't reduce system to a polynomial in one variable.",
                "error.algsys_failed",
            ),
        ];
        for (text, id) in cases {
            let info = enhance_error(&format!("{text}{hint}"), &catalog())
                .unwrap_or_else(|| panic!("no explanation for {text}"));
            assert_eq!(info.id, id, "{text}");
        }
        let info =
            enhance_error("allroots: expected a polynomial; found sin(x)", &catalog()).unwrap();
        assert_eq!(info.explanation.params["found"], "sin(x)");
        assert_eq!(
            info.explanation.text,
            "`allroots` works only on polynomials, but got `sin(x)`."
        );
        let info = enhance_error("inpart: fell off the end.", &catalog()).unwrap();
        assert_eq!(info.explanation.params["func"], "inpart");
    }
}
