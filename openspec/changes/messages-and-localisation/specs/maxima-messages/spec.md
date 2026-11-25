## Purpose

How Aximar explains Maxima's errors, notes and questions: stable message IDs with named parameters, questions that stop a cell instead of hanging it, Lisp errors that keep their cause, notes in plain language, and an output in every failed cell.

## ADDED Requirements

### Requirement: Friendly messages have stable IDs

Every friendly message Aximar shows for Maxima's output (an error explanation, a note, a question) SHALL have a stable ID and named parameters, and SHALL be an entry in Aximar's translation catalog whose context is the ID and whose source text is the English template. Errors and notes SHALL carry the ID, the parameters and the English text rendered from the template. `docs/maxima-messages.md` SHALL list every ID with the Maxima text it covers, its parameters and its English text, and SHALL be generated from the catalog. An ID SHALL NOT change meaning once released.

#### Scenario: An explanation carries its ID

- **WHEN** a cell fails with `expt: undefined: 0 to a negative exponent.`
- **THEN** its `error_info` SHALL have `id` `error.division_by_zero`, and its `title` and `explanation` SHALL be the catalog's English text for that ID

#### Scenario: Parameters fill the text

- **WHEN** a cell fails with `Too many arguments supplied to f(x); found: [1,2]`
- **THEN** its `error_info` SHALL have `id` `error.arg_count` with parameters naming `f`, its definition `f(x)` and the arguments `[1,2]`, and the English text SHALL contain them

#### Scenario: The catalog and the code agree

- **WHEN** the unit tests run
- **THEN** every ID the code can produce SHALL have an entry in the catalog's template, every placeholder in an entry SHALL be a parameter the code sets, and `docs/maxima-messages.md` SHALL match what the generator produces from the template

### Requirement: Syntax errors are explained

When a cell fails with one of Maxima's syntax errors, its `error_info` SHALL explain it with an ID for the parser message it matches (such as a missing parenthesis, or input that ends too early), with the parser's details as parameters. A parser message Aximar doesn't recognise SHALL get `error.syntax` with Maxima's text as a parameter.

#### Scenario: Missing parenthesis (needs Maxima)

- **WHEN** a cell runs `f(x;`
- **THEN** its `error_info` SHALL have an ID for a missing delimiter, with `)` as a parameter

#### Scenario: Input ends too early (needs Maxima)

- **WHEN** a cell runs `1+;`
- **THEN** its `error_info` SHALL have `id` `error.incomplete_input`

### Requirement: Questions stop the cell with the fix

When Maxima asks a question during an evaluation, the cell SHALL stop with an error output quoting the question, and its explanation SHALL name a line that answers it before the computation. No answer SHALL be sent to Maxima, and the session SHALL be ready for the next cell. This SHALL hold for every kind of question, in the window, over MCP and in batch runs, on both evaluation paths.

| Question | ID | Suggested line |
|---|---|---|
| Is e positive, negative or zero? | `question.sign` | `assume(e > 0)` |
| Is e positive or zero? / negative or zero? / positive or negative? | `question.sign_pz` / `question.sign_nz` / `question.sign_pn` | `assume(e > 0)` / `assume(e < 0)` / `assume(e > 0)` |
| Is e zero or nonzero? | `question.zero` | `assume(notequal(e, 0))` |
| Is a equal to b? | `question.equal` | `assume(notequal(a, b))` |
| Is e an integer? (or another property) | `question.property` | `declare(e, integer)` |
| anything else | `question.other` | none |

#### Scenario: Sign question (needs Maxima)

- **WHEN** a cell runs `integrate(exp(-a*x), x, 0, inf);` with nothing assumed about `a`
- **THEN** the cell SHALL have an error output whose text contains "Is a positive, negative or zero?", with `error_info.id` `question.sign` and `assume(a > 0)` in its suggestion, within a few seconds

#### Scenario: Yes/no question (needs Maxima)

- **WHEN** a cell runs `integrate(x^a, x);`
- **THEN** the cell SHALL have an error output with `error_info.id` `question.equal` and parameters `a` and `-1`, well before the evaluation timeout

#### Scenario: Nothing leaks into the next cell (needs Maxima)

- **WHEN** a cell asks a question and the next cell runs `1/0;`
- **THEN** the second cell's error text SHALL be Maxima's division error alone, with no answer text such as `positive` in it

#### Scenario: Questions inside errcatch (needs Maxima)

- **WHEN** a cell runs `errcatch(integrate(exp(-a*x), x, 0, inf));`
- **THEN** the cell SHALL succeed with the result `[]`

#### Scenario: Over MCP (needs Maxima)

- **WHEN** an MCP client runs a cell that asks a question
- **THEN** `run_cell` SHALL return a result with `is_error` true and the question in `error`, not a tool error

### Requirement: Every failure leaves an output in its cell

A cell whose evaluation fails, times out or is cancelled SHALL have an error output explaining why, as well as its error status. The window SHALL show it in the cell.

#### Scenario: Timeout (needs Maxima)

- **WHEN** a cell runs longer than the evaluation timeout
- **THEN** the cell SHALL have an error output with `error_info.id` `error.timeout` and the timeout in seconds as a parameter

#### Scenario: Cancelled (needs Maxima)

- **WHEN** the user cancels a running cell
- **THEN** the cell SHALL have an error output with `error_info.id` `error.cancelled`

### Requirement: Lisp errors keep their message

When Maxima reports a Lisp error, the cell's error text SHALL contain the whole condition, joined onto one line, without SBCL's "Automatically continuing." and "To enable the Lisp debugger" lines. Known conditions SHALL be explained: a value of the wrong type (`error.lisp_type`), runaway recursion (`error.recursion`) and floating-point overflow (`error.float_overflow`); others SHALL get `error.lisp` with the condition text as a parameter.

#### Scenario: Wrong type (needs Maxima)

- **WHEN** a cell runs `?car(1);`
- **THEN** the error text SHALL contain "The value 1 is not of type LIST", and `error_info.id` SHALL be `error.lisp_type` with the value `1` and the type in words

#### Scenario: Runaway recursion (needs Maxima)

- **WHEN** a cell runs `g(n) := g(n + 1)$ g(1);`
- **THEN** `error_info.id` SHALL be `error.recursion`, and the next cell SHALL run normally

### Requirement: Error text leaves out the debugger hint

Error text SHALL NOT contain Maxima's line ` -- an error. To debug this try: debugmode(true);`, on either evaluation path.

#### Scenario: Divergent integral (needs Maxima)

- **WHEN** a cell runs `integrate(1/x, x, 0, 1);`
- **THEN** its error text SHALL be "defint: integral is divergent." with no debugger hint

### Requirement: Notes read as plain language

Recognised notes in a cell's text output SHALL be reported as notes, each with an ID, parameters, English text and the range of text-output lines it covers. The window SHALL show each note as one line in a note style in place of Maxima's lines, with Maxima's lines one click away, and SHALL show unrecognised text as Maxima wrote it. Consecutive `rat: replaced` lines SHALL form one note. Text output, notes included, SHALL show before plots.

#### Scenario: Decimals replaced (needs Maxima)

- **WHEN** a cell runs `rat(0.5*x + 0.25);`
- **THEN** the cell SHALL have one note with ID `note.rat_replaced` covering both `rat: replaced` lines and naming `0.25 → 1/4` and `0.5 → 1/2`, and the result SHALL be unchanged

#### Scenario: Solutions may be lost (needs Maxima)

- **WHEN** a cell runs `solve(sin(x) = 1/2, x);`
- **THEN** the cell SHALL have a note with ID `note.solve_arc_trig` covering both of Maxima's lines

#### Scenario: Print text stays text

- **WHEN** a cell prints `rat: my own label` with `print`
- **THEN** the cell SHALL have no note, and the window SHALL show the text unchanged

#### Scenario: A passing division by zero is a note (needs Maxima)

- **WHEN** a cell runs `quad_qags(1/sqrt(abs(x)), x, -1, 1);`
- **THEN** the `expt: undefined` line SHALL be a `note.zero_power` note and the cell SHALL not be an error

#### Scenario: The window shows Maxima's lines on request

- **WHEN** the user clicks a note line's toggle
- **THEN** the window SHALL show the Maxima lines the note covers in its place

### Requirement: Loaded outputs are explained the same way

When a notebook is opened, Aximar SHALL compute `error_info` and notes for its outputs from the saved text, as it does after an evaluation. Files SHALL NOT store notes or explanations.

#### Scenario: Reopened note

- **WHEN** a notebook saved with a `rat: replaced` stream output is opened
- **THEN** the loaded output SHALL have the same `note.rat_replaced` note an evaluation gives, and the file written back SHALL hold only the stream text
