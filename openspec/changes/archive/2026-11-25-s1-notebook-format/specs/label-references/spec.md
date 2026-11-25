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

### Requirement: A client's cells follow the window's label rules

In simple mode, `evaluate_expression` SHALL accept an optional cell ID and the IDs of the code cells above it, for a client that keeps its notebook itself. With them, the expression SHALL run as that cell: it SHALL get the session's next gutter number, returned as `execution_count`; `%oN` SHALL mean the run numbered N in this session; and a bare `%` at its start SHALL mean the output of the nearest cell above whose latest run in this session produced one. Without them, the expression SHALL take no number and keep Maxima's own labels: `%oN` and a bare `%` SHALL reach Maxima as typed, as in a plain Maxima session.

#### Scenario: Labels and the output above (needs Maxima)

- **WHEN** client cells `a` (`10;`), `b` (`5;`) and `c` (`1/0;`) run in order, then cell `d` runs `%o1 + %;` with `a`, `b` and `c` above it
- **THEN** `d` SHALL get `execution_count` 4 and the result 15

#### Scenario: A cell that hasn't run in this session (needs Maxima)

- **WHEN** a client cell runs `%;` with only a cell above it that this session never ran, such as one reopened from a file
- **THEN** it SHALL fail as a label with no value

#### Scenario: Unknown label (needs Maxima)

- **WHEN** a client cell runs `%o2 + 1;` and no run numbered 2 exists in this session
- **THEN** the result SHALL be an error "Out[2] (%o2) has no value in this session", with the cell's `execution_count`

#### Scenario: A new session numbers from 1

- **WHEN** the session restarts
- **THEN** the next client cell SHALL get `execution_count` 1, and no cell from before the restart SHALL be an output above

#### Scenario: A plain expression in simple mode (needs Maxima)

- **WHEN** a client evaluates `10;` and then `%oK + 1;`, where `%oK` is the output label the first result reported, both without a cell ID
- **THEN** the second result SHALL be 11

#### Scenario: Clients can tell whether the server runs client cells

- **WHEN** a client lists the server's tools
- **THEN** simple mode's `evaluate_expression` input schema SHALL include `cell_id` and `cells_above`
