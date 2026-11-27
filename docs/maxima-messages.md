<!-- Generated from locales/messages.pot by scripts/i18n-extract. Do not edit. -->

# Aximar's messages about Maxima's output

Each explanation Aximar gives for something Maxima printed, by its ID. The ID is stable: MCP results carry it as `error_info.id`, and each field's text is a catalog entry whose context is the ID and the field. Parameters fill the placeholders in braces. See [i18n.md](i18n.md).

## `error.algsys_failed`

Covers: "algsys: Couldn't reduce system to a polynomial in one variable." screen: errors--all-explanations

| Field | English |
|---|---|
| title | Can't Solve the System |
| explanation | algsys, which solve uses for systems, couldn't turn the equations into a polynomial in one variable. |
| suggestion | For a numerical answer, try mnewton() from load(mnewton) with a starting point. |

## `error.arg_count`

Covers: "Too many arguments supplied to f(x); found: [1, 2]" or "Too few ..." screen: errors--all-explanations

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

## `error.assign_target`

Covers: "assignment: cannot assign to 2" screen: errors--all-explanations

Parameters: `target`

| Field | English |
|---|---|
| title | Can't Assign |
| explanation | The left side of : must be a variable name, but it is {target}. |
| suggestion | To write an equation, use = instead of :, as in x^2 = 4. |

## `error.cancelled`

Covers: the user stopped a running cell screen: errors--all-explanations

| Field | English |
|---|---|
| title | Cancelled |
| explanation | The cell was stopped before it finished. |

## `error.divergent_integral`

Covers: "defint: integral is divergent." screen: errors--all-explanations

| Field | English |
|---|---|
| title | Divergent Integral |
| explanation | The definite integral does not converge to a finite value. |
| suggestion | Check the integration bounds and integrand. The function may have singularities or not decay fast enough. |

## `error.division_by_zero`

Covers: "expt: undefined: 0 to a negative exponent." or "Division by 0" screen: cells--bracket-selection screen: cells--notebook screen: cells--selection screen: errors--all-explanations screen: errors--translated-maxima-error screen: screens--drop-mixed screen: screens--drop-notebooks screen: screens--drop-refused screen: screens--whole-window

| Field | English |
|---|---|
| title | Division by Zero |
| explanation | The expression involves division by zero or raising zero to a negative power, which is undefined. |
| suggestion | Check your denominator or exponent for cases where the value might be zero. |

## `error.empty_list`

Covers: "first: empty argument.", or the same from last and others screen: errors--all-explanations

Parameters: `func`

| Field | English |
|---|---|
| title | Empty List |
| explanation | {func} was given an empty list, which has no elements to take. |
| suggestion | Check where the list comes from, for example a solve() that found no solutions. |

## `error.float_overflow`

Covers: "Maxima encountered a Lisp error: arithmetic error FLOATING-POINT-OVERFLOW signalled" screen: errors--all-explanations

| Field | English |
|---|---|
| title | Number Too Large |
| explanation | A floating-point result was too large to represent. |
| suggestion | Keep the computation exact, or use bfloat() for very large numbers. |

## `error.incomplete_input`

Covers: "incorrect syntax: Premature termination of input at ;." screen: errors--all-explanations

| Field | English |
|---|---|
| title | Incomplete Expression |
| explanation | The expression ended unexpectedly. This usually means a missing closing parenthesis, bracket, or semicolon. |
| suggestion | Check that all parentheses () and brackets [] are matched, and the expression ends with ; or $ |

## `error.inconsistent_equations`

Covers: "solve: inconsistent equations" and similar screen: errors--all-explanations

| Field | English |
|---|---|
| title | Inconsistent Equations |
| explanation | The system of equations has no solution: the equations contradict each other. |
| suggestion | Check that the equations are correct and compatible. |

## `error.lisp`

Covers: "Maxima encountered a Lisp error:" and any other condition screen: errors--all-explanations

Parameters: `condition`

| Field | English |
|---|---|
| title | Internal Error |
| explanation | Maxima encountered an internal error in the underlying Lisp system: {condition} |
| suggestion | Try restarting the Maxima session. If the error persists, the expression may be too complex or trigger a bug in Maxima. |

## `error.lisp_type`

Covers: "Maxima encountered a Lisp error: The value 1 is not of type LIST" screen: errors--all-explanations

Parameters: `lisp_type`, `value`

| Field | English |
|---|---|
| title | Wrong Kind of Value |
| explanation | A function inside Maxima expected a value of type {lisp_type} but got {value}. |
| suggestion | Check the arguments of the functions in this cell, especially Lisp functions called with ?, which don't check their arguments. |

## `error.log_zero`

Covers: "log: encountered log(0)." screen: errors--all-explanations

| Field | English |
|---|---|
| title | Logarithm of Zero |
| explanation | The expression takes the logarithm of 0, which is undefined. |
| suggestion | Check the argument of log for values that make it 0, or take a limit instead. |

## `error.matrix_dimension`

Covers: "matrix: all rows must be the same length." or "incompatible dimensions" screen: errors--all-explanations

| Field | English |
|---|---|
| title | Matrix Dimension Error |
| explanation | Matrix rows have different lengths, or the matrices have incompatible dimensions for the operation. |
| suggestion | Ensure all rows have the same number of elements and matrices have compatible dimensions. |

## `error.not_polynomial`

Covers: "allroots: expected a polynomial; found sin(x)" screen: errors--all-explanations

Parameters: `found`, `func`

| Field | English |
|---|---|
| title | Not a Polynomial |
| explanation | {func} works only on polynomials, but got {found}. |
| suggestion | For other equations, try find_root() over an interval, or solve(). |

## `error.package_not_found`

Covers: "loadfile: failed to load x" or "Cannot find file x" screen: errors--all-explanations

Parameters: `package`

| Field | English |
|---|---|
| title | Package Not Found |
| explanation | The package "{package}" could not be found or loaded. |
| explanation.unnamed | The package could not be found or loaded. |
| suggestion | Check the package name. Common packages: draw, simplex, lsquares, descriptive, distrib. |

## `error.part_atom`

Covers: "part: argument must be a non-atomic expression; found x" screen: errors--all-explanations

Parameters: `found`, `func`

| Field | English |
|---|---|
| title | No Parts to Take |
| explanation | {func} needs an expression with parts, such as a list or a sum, but got {found}. |
| suggestion | Check that the variable holds the list or expression you meant. |

## `error.part_index`

Covers: "part: fell off the end.", or the same from inpart or rest screen: errors--all-explanations

Parameters: `func`

| Field | English |
|---|---|
| title | Index Out of Range |
| explanation | {func} asked for a part beyond the end of the expression or list. |
| suggestion | Check the index against length() of the expression or list. |

## `error.recursion`

Covers: "Maxima encountered a Lisp error: Control stack exhausted ..." screen: errors--all-explanations

| Field | English |
|---|---|
| title | Runaway Recursion |
| explanation | A function called itself, directly or through others, until Maxima ran out of room for the calls. |
| suggestion | Check that each recursive function has a case that stops it, such as f(0) := 1. |

## `error.syntax`

Covers: any other "incorrect syntax: ..." message screen: errors--all-explanations

Parameters: `message`

| Field | English |
|---|---|
| title | Syntax Error |
| explanation | The expression contains a syntax error: {message}. Check for missing operators, unmatched parentheses, or misplaced characters. |
| suggestion | Check for missing *, +, or other operators. Ensure all parentheses are matched. |

## `error.syntax_missing`

Covers: "incorrect syntax: Missing )", or another closing delimiter screen: errors--all-explanations

Parameters: `delimiter`

| Field | English |
|---|---|
| title | Syntax Error |
| explanation | A closing {delimiter} is missing. |
| suggestion | Check that every opening bracket has its closing {delimiter}, in the right order. |

## `error.syntax_operator`

Covers: "incorrect syntax: y is not an infix operator" or "! is not a prefix operator" screen: errors--all-explanations

Parameters: `token`

| Field | English |
|---|---|
| title | Syntax Error |
| explanation.infix | Two values appear next to each other without an operator between them, before "{token}". |
| explanation.prefix | "{token}" can't start an expression here. Check for a missing operand or a misplaced operator. |
| suggestion | Check for missing *, +, or other operators. Ensure all parentheses are matched. |

## `error.timeout`

Covers: Aximar stopped a cell that ran longer than the evaluation timeout screen: errors--all-explanations

Parameters: `seconds`

| Field | English |
|---|---|
| title | Timed Out |
| explanation | The cell ran for longer than {seconds} seconds, so Aximar stopped it. |
| suggestion | Raise the evaluation timeout in Settings, or try a smaller or numeric version of the computation. |

## `error.undefined_variable`

Covers: "unbound variable x" or "undefined variable x" screen: errors--all-explanations

Parameters: `name`

| Field | English |
|---|---|
| title | Undefined Variable |
| explanation | The variable "{name}" has not been assigned a value. |
| suggestion | Assign it first with {name}: value; or use it as a symbolic variable. |

## `error.unknown_function`

Covers: "funcall: no such function: f" or "The function f is not known to Maxima" screen: errors--all-explanations

Parameters: `func`, `package`

| Field | English |
|---|---|
| title | Unknown Function |
| explanation | "{func}" is not a known Maxima function. |
| explanation.in_package | "{func}" is provided by the "{package}" package, which needs to be loaded first. |
| suggestion | Check the spelling or use describe("{func}") to search. |
| suggestion.in_package | Add load("{package}")$ before using this function. |

## `note.plot_non_numeric`

Covers: "plot2d: expression evaluates to non-numeric value somewhere in plotting range." function is plot2d or plot3d screen: notes--all-notes

Parameters: `function`

| Field | English |
|---|---|
| text | {function} left out points where the expression has no real value. |

## `note.principal_value`

Covers: "Principal Value", from integrate screen: notes--all-notes

| Field | English |
|---|---|
| text | The integral diverges inside the range, so the result is its Cauchy principal value. |

## `note.quadrature_undefined`

Covers: "quad_qags: Numerical evaluation of f at 0.0 is not a float or is not defined". function is a quadrature function; expr a Maxima expression; point a number screen: notes--all-notes

Parameters: `expr`, `function`, `point`

| Field | English |
|---|---|
| text | {function} couldn't evaluate {expr} at {point}, so the result may be inaccurate. |

## `note.rat_replaced`

Covers: "rat: replaced 0.5 by 1/2 = 0.5", one line per number. replacements lists them, such as "0.25 → 1/4, 0.5 → 1/2" screen: cells--bracket-selection screen: cells--notebook screen: cells--selection screen: notes--all-notes screen: screens--drop-mixed screen: screens--drop-notebooks screen: screens--drop-refused screen: screens--whole-window

Parameters: `count`, `examples`, `replacements`

| Field | English |
|---|---|
| text | Maxima replaced decimals with exact fractions: {replacements}. |
| text.many | Maxima replaced {count} decimals with exact fractions, such as {examples}. Add ratprint: false$ to hide these messages. |

## `note.solve_arc_trig`

Covers: "solve: using arc-trig functions to get a solution. Some solutions will be lost." screen: notes--all-notes

| Field | English |
|---|---|
| text | solve used inverse trigonometric functions, so it gives only some of the solutions. |

## `note.solve_dependent`

Covers: "solve: dependent equations eliminated: (2)". equations lists their positions, such as (2) screen: notes--all-notes

Parameters: `equations`

| Field | English |
|---|---|
| text | solve dropped equations that follow from the others: {equations}. |

## `note.tellsimp_noncommutative`

Covers: "tellsimp: warning: rule will treat '+' as noncommutative and nonassociative." screen: notes--all-notes

| Field | English |
|---|---|
| text | tellsimp will treat the rule's main operator as noncommutative and nonassociative, so the rule matches only terms in the order it was written. |

## `note.zero_power`

Covers: "expt: undefined: 0 to a negative exponent.", in a cell that still has a result screen: notes--all-notes

| Field | English |
|---|---|
| text | Maxima met 0 to a negative power on the way, and carried on without that part. |

## `question.equal`

Covers: "Maxima needs to know: Is a equal to -1?" screen: errors--all-explanations

Parameters: `a`, `b`, `line`

| Field | English |
|---|---|
| title | Maxima Needs to Know |
| explanation | Maxima needs to know whether {a} equals {b} to go on. |
| suggestion | Add {line} before this line, or in a cell above, and run it again. |

## `question.other`

Covers: "Maxima needs to know: ..." with any other question screen: errors--all-explanations

Parameters: `question`

| Field | English |
|---|---|
| title | Maxima Needs to Know |
| explanation | Maxima asked "{question}" and can't get an answer from a cell. |
| suggestion | Use assume() or declare() before this line to tell Maxima what it needs, then run the cell again. |

## `question.property`

Covers: "Maxima needs to know: Is n an integer?", or another property screen: errors--all-explanations

Parameters: `expr`, `line`, `property`

| Field | English |
|---|---|
| title | Maxima Needs to Know |
| explanation | Maxima needs to know whether {expr} has the property {property} to go on. |
| suggestion | Add {line} before this line, or in a cell above, and run it again. |

## `question.sign`

Covers: "Maxima needs to know: Is a positive, negative or zero?" screen: errors--all-explanations

Parameters: `expr`, `line`

| Field | English |
|---|---|
| title | Maxima Needs to Know |
| explanation | Maxima needs to know whether {expr} is positive, negative or zero to go on. |
| suggestion | Add {line} before this line, or in a cell above, and run it again. |

## `question.sign_nz`

Covers: "Maxima needs to know: Is a negative or zero?" screen: errors--all-explanations

Parameters: `expr`, `line`

| Field | English |
|---|---|
| title | Maxima Needs to Know |
| explanation | Maxima needs to know whether {expr} is negative or zero to go on. |
| suggestion | Add {line} before this line, or in a cell above, and run it again. |

## `question.sign_pn`

Covers: "Maxima needs to know: Is k positive or negative?" screen: errors--all-explanations

Parameters: `expr`, `line`

| Field | English |
|---|---|
| title | Maxima Needs to Know |
| explanation | Maxima needs to know whether {expr} is positive or negative to go on. |
| suggestion | Add {line} before this line, or in a cell above, and run it again. |

## `question.sign_pz`

Covers: "Maxima needs to know: Is a positive or zero?" screen: errors--all-explanations

Parameters: `expr`, `line`

| Field | English |
|---|---|
| title | Maxima Needs to Know |
| explanation | Maxima needs to know whether {expr} is positive or zero to go on. |
| suggestion | Add {line} before this line, or in a cell above, and run it again. |

## `question.zero`

Covers: "Maxima needs to know: Is x zero or nonzero?" screen: errors--all-explanations

Parameters: `expr`, `line`

| Field | English |
|---|---|
| title | Maxima Needs to Know |
| explanation | Maxima needs to know whether {expr} is zero to go on. |
| suggestion | Add {line} before this line, or in a cell above, and run it again. |
