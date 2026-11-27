<!-- Generated from locales/messages.pot by scripts/i18n-extract. Do not edit. -->

# Aximar's messages about Maxima's output

Each explanation Aximar gives for something Maxima printed, by its ID. The ID is stable: MCP results carry it as `error_info.id`, and each field's text is a catalog entry whose context is the ID and the field. Parameters fill the placeholders in braces. See [i18n.md](i18n.md).

## `error.arg_count`

Covers: "Too many arguments supplied to f(x); found: [1, 2]" or "Too few ..."

Parameters: `definition`, `found`, `func`, `signatures`

| Field | English |
|---|---|
| title | Wrong Argument Count |
| explanation | {func} is defined as {definition}, but was called with {found}. |
| explanation.too_few | Too few arguments were passed to {func}. |
| explanation.too_many | Too many arguments were passed to {func}. |
| suggestion | Call {func} with the arguments {definition} names, or change its definition. |
| suggestion.describe | Check the documentation for {func} with describe({func}). |
| suggestion.signatures | Correct usage: {signatures} |

## `error.divergent_integral`

Covers: "defint: integral is divergent."

| Field | English |
|---|---|
| title | Divergent Integral |
| explanation | The definite integral does not converge to a finite value. |
| suggestion | Check the integration bounds and integrand. The function may have singularities or not decay fast enough. |

## `error.division_by_zero`

Covers: "expt: undefined: 0 to a negative exponent." or "Division by 0"

| Field | English |
|---|---|
| title | Division by Zero |
| explanation | The expression involves division by zero or raising zero to a negative power, which is undefined. |
| suggestion | Check your denominator or exponent for cases where the value might be zero. |

## `error.incomplete_input`

Covers: "incorrect syntax: Premature termination of input at ;."

| Field | English |
|---|---|
| title | Incomplete Expression |
| explanation | The expression ended unexpectedly. This usually means a missing closing parenthesis, bracket, or semicolon. |
| suggestion | Check that all parentheses () and brackets [] are matched, and the expression ends with ; or $ |

## `error.inconsistent_equations`

Covers: "solve: inconsistent equations" and similar

| Field | English |
|---|---|
| title | Inconsistent Equations |
| explanation | The system of equations has no solution: the equations contradict each other. |
| suggestion | Check that the equations are correct and compatible. |

## `error.lisp`

Covers: "Maxima encountered a Lisp error:" and the condition

| Field | English |
|---|---|
| title | Internal Error |
| explanation | Maxima encountered an internal error in the underlying Lisp system. |
| suggestion | Try restarting the Maxima session. If the error persists, the expression may be too complex or trigger a bug in Maxima. |

## `error.matrix_dimension`

Covers: "matrix: all rows must be the same length." or "incompatible dimensions"

| Field | English |
|---|---|
| title | Matrix Dimension Error |
| explanation | Matrix rows have different lengths, or the matrices have incompatible dimensions for the operation. |
| suggestion | Ensure all rows have the same number of elements and matrices have compatible dimensions. |

## `error.package_not_found`

Covers: "loadfile: failed to load x" or "Cannot find file x"

Parameters: `package`

| Field | English |
|---|---|
| title | Package Not Found |
| explanation | The package "{package}" could not be found or loaded. |
| explanation.unnamed | The package could not be found or loaded. |
| suggestion | Check the package name. Common packages: draw, simplex, lsquares, descriptive, distrib. |

## `error.syntax`

Covers: any other "incorrect syntax: ..." message

Parameters: `message`

| Field | English |
|---|---|
| title | Syntax Error |
| explanation | The expression contains a syntax error: {message}. Check for missing operators, unmatched parentheses, or misplaced characters. |
| suggestion | Check for missing *, +, or other operators. Ensure all parentheses are matched. |

## `error.syntax_missing`

Covers: "incorrect syntax: Missing )", or another closing delimiter

Parameters: `delimiter`

| Field | English |
|---|---|
| title | Syntax Error |
| explanation | A closing {delimiter} is missing. |
| suggestion | Check that every opening bracket has its closing {delimiter}, in the right order. |

## `error.syntax_operator`

Covers: "incorrect syntax: y is not an infix operator" or "! is not a prefix operator"

Parameters: `token`

| Field | English |
|---|---|
| title | Syntax Error |
| explanation.infix | Two values appear next to each other without an operator between them, before "{token}". |
| explanation.prefix | "{token}" can't start an expression here. Check for a missing operand or a misplaced operator. |
| suggestion | Check for missing *, +, or other operators. Ensure all parentheses are matched. |

## `error.undefined_variable`

Covers: "unbound variable x" or "undefined variable x"

Parameters: `name`

| Field | English |
|---|---|
| title | Undefined Variable |
| explanation | The variable "{name}" has not been assigned a value. |
| suggestion | Assign it first with {name}: value; or use it as a symbolic variable. |

## `error.unknown_function`

Covers: "funcall: no such function: f" or "The function f is not known to Maxima"

Parameters: `func`, `package`

| Field | English |
|---|---|
| title | Unknown Function |
| explanation | "{func}" is not a known Maxima function. |
| explanation.in_package | "{func}" is provided by the "{package}" package, which needs to be loaded first. |
| suggestion | Check the spelling or use describe("{func}") to search. |
| suggestion.in_package | Add load("{package}")$ before using this function. |

## `question.other`

Covers: Maxima asked a question, such as "Is x positive, negative or zero?"

| Field | English |
|---|---|
| title | Assumption Required |
| explanation | Maxima needs to know a property of a variable to proceed with the computation. |
| suggestion | Use assume() to declare properties, e.g.: assume(x > 0); |
