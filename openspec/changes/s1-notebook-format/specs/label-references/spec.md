## ADDED Requirements

### Requirement: Each session numbers its runs from 1

A notebook's runs SHALL be numbered from 1 in each Maxima session, and from 1 after a notebook is loaded from a file, whatever numbers the loaded outputs show. The VS Code extension SHALL follow the same rule.

#### Scenario: Running a loaded notebook

- **WHEN** a notebook whose last output shows `Out[7]` is opened and its first code cell runs
- **THEN** that run SHALL be `In[1]`

### Requirement: Labels on loaded outputs have no value

A `%oN` or `%iN` that names a number shown only on a stale output SHALL fail as a label with no value, and a bare `%` SHALL skip stale outputs.

#### Scenario: Reference to a loaded output

- **WHEN** a notebook whose cell shows `Out[3]` from the file is opened, and a new cell runs `%o3 + 1;` before any cell has run
- **THEN** the cell SHALL fail with "Out[3] (%o3) has no value in this session"

### Requirement: An expression can bring its own label context

`evaluate_expression` SHALL accept an optional label context, a map from gutter numbers to Maxima labels and the previous output's label. With it, the expression SHALL be prepared as a cell with that context, bare `%` included; without it, the expression SHALL use the notebook's labels and leave a bare `%` to Maxima.

#### Scenario: The VS Code extension resolves labels on the server

- **WHEN** an expression `%o1 + %` is evaluated with the context `{1: "%o167"}` and previous output `%o170`
- **THEN** Maxima SHALL receive `%o167 + %o170`

#### Scenario: Unknown label in the context

- **WHEN** an expression `%o2 + 1` is evaluated with the context `{1: "%o167"}`
- **THEN** it SHALL fail with "Out[2] (%o2) has no value in this session"
