# label-references Specification

## Purpose

How a cell refers to earlier results. The gutter numbers each run of a code cell as `In[n]`/`Out[n]`. Maxima numbers its own labels differently, because Aximar runs setup code and housekeeping statements in the same session, so Aximar rewrites a cell's references before sending it to Maxima. These rules make a reference mean what the gutter shows, or fail with a clear error.

The rewriting lives in `crates/aximar-core/src/maxima/labels.rs` and runs in `evaluation::evaluate_cell`, so the GUI and MCP behave the same. Maxima's `%th(n)`, `_` and `__` are not covered yet (#61).

## Requirements

### Requirement: Display labels follow the gutter

A reference `%oN` or `%iN` in a cell SHALL mean the output or input of the run numbered `N` in the gutter, in the current Maxima session. For a cell with several statements, it SHALL mean the cell's last statement.

#### Scenario: Output labels

- **WHEN** cells `a: 10;` and `b: 20$` run as `In[1]` and `In[2]`, and a later cell runs `[%o1, %o2];`
- **THEN** the result SHALL be `[10, 20]` (needs Maxima)

#### Scenario: Input label

- **WHEN** `a: 10;` ran as `In[1]` and a later cell runs `%i1;`
- **THEN** the result SHALL be the input expression `a:10` (needs Maxima)

#### Scenario: Labels in every statement

- **WHEN** `Out[1]` has a value and a cell runs `x: %o1; y: %o1 + 1;`
- **THEN** both references SHALL be rewritten to `Out[1]`'s label

### Requirement: A label with no value is an error

A `%oN` or `%iN` whose run has no recorded label in the current session SHALL make the cell fail with an error naming the label, and the cell SHALL NOT be sent to Maxima. This covers a run that ended in an error, a cell loaded from a file, and a number that hasn't run in this session.

#### Scenario: Reference to a cell that failed

- **WHEN** `In[3]` was `1/0;`, which failed, and a later cell runs `%o3 + 1;`
- **THEN** the cell SHALL show the error "Out[3] (%o3) has no value in this session. Run that cell again and use its new number." (needs Maxima)

#### Scenario: Input label with no value

- **WHEN** no run numbered 3 exists in the session and a cell runs `%i3 + 1;`
- **THEN** the cell SHALL fail with an error starting "In[3] (%i3) has no value in this session"

### Requirement: Bare % in the first statement means the previous cell

A bare `%` in a cell's first statement SHALL mean the output of the nearest cell above it whose output came from a successful run in the current session. Cells that ended in an error, and outputs left from an earlier session, SHALL be skipped. The first statement ends at the first `;` or `$` outside strings and comments.

#### Scenario: Skipping a cell that failed

- **WHEN** cells `a: 10;`, `b: 20$` and `1/0;` run in that order, and the next cell runs `%;`
- **THEN** the result SHALL be `20` (needs Maxima)

#### Scenario: No earlier output

- **WHEN** no cell above has an output from the current session and a cell runs `% + 1;`
- **THEN** the cell SHALL fail with an error starting "% refers to the previous cell's output"

### Requirement: Bare % in later statements is Maxima's own

A bare `%` after a cell's first statement SHALL be left for Maxima, where it means the result of the statement before it.

#### Scenario: Two statements in one cell

- **WHEN** a cell `100;` has run and the next cell runs `x: 5; % + 1;`
- **THEN** the result SHALL be `6` (needs Maxima)

#### Scenario: No previous cell needed

- **WHEN** no cell above has an output and a cell runs `x: 5; % + 1;`
- **THEN** the cell SHALL run, and its result SHALL be `6`

### Requirement: Strings and comments are left as typed

`%`, `%oN` and `%iN` inside string literals and comments SHALL NOT be rewritten or checked. Strings use `\` to escape the next character, and comments nest, as in Maxima.

#### Scenario: Percent in a string

- **WHEN** `Out[1]` has a value and a cell runs `print("100% done", %);`
- **THEN** the string SHALL stay `"100% done"` and only the second `%` SHALL be rewritten

#### Scenario: Labels in a nested comment

- **WHEN** a cell runs `/* uses % and %o1 /* nested % */ */ % + %o1;`
- **THEN** the comment SHALL stay as typed and only the code after it SHALL be rewritten

### Requirement: Labels inside longer names are not labels

A `%oN` or `%iN` that is part of a longer name, such as `x%o1` or `%o1a`, SHALL NOT be treated as a label.

#### Scenario: Longer names

- **WHEN** a cell runs `x%o1 + %o1y;` and no run is numbered 1
- **THEN** the input SHALL be sent to Maxima unchanged

### Requirement: Numbering restarts with the Maxima session

When a new Maxima process starts for a notebook, its execution numbers SHALL start again at 1, and every label recorded in the earlier session SHALL be forgotten. Running the cells again in order SHALL give them the same `In[n]`/`Out[n]` as before, so references between them keep working. A cell not yet run in the new session MAY still show its old number, but its output SHALL NOT be used to resolve a reference.

#### Scenario: Restart then run all

- **WHEN** cells `a: 10;`, `%o1 + 1;` and `%i1;` have run, the session restarts, and all cells run again in order
- **THEN** they SHALL be numbered 1, 2 and 3 again, and the second and third SHALL give `11` and `a:10` (needs Maxima)

#### Scenario: Reference before rerunning

- **WHEN** `a: 10;` ran as `In[1]`, the session restarts, and the first cell run in the new session is `%o1;`
- **THEN** that cell SHALL fail with "Out[1] (%o1) has no value in this session" (needs Maxima)

#### Scenario: Stale output with a reused number

- **WHEN** cells A and B ran as `In[1]` and `In[2]`, the session restarts, and B runs first, becoming `In[1]`
- **THEN** a bare `%` in the cell after B SHALL mean B's new output, and a bare `%` in B SHALL find no earlier output, because A's `Out[1]` belongs to the earlier session
