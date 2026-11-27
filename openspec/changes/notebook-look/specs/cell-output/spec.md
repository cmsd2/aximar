## Purpose

How a code cell's output and state look in the notebook window, in bracket and card style alike: results, printed text, notes, errors, suggestions, and whether the cell is running, queued or silent.

## ADDED Requirements

### Requirement: An error shows one red signal

A cell whose run failed SHALL show that it failed on its bracket in bracket style, or its border in card style, in the error colour. In its output, the only red SHALL be a small error icon: no red text, red background or red bar. An explained error SHALL show its title in the normal text colour after the error icon, its explanation in the secondary text colour, and its suggestion after a lightbulb icon, with no coloured box or bar. A button labelled "Show Maxima's error" SHALL show Maxima's own text in monospace. An error Aximar can't explain SHALL show Maxima's text in monospace, in the normal text colour, after the same red error icon, with no coloured box.

#### Scenario: Division by zero

- **WHEN** a cell's output carries the `error.division_by_zero` explanation
- **THEN** the title "Division by Zero" is in the normal text colour, the small icon before it is red, the suggestion has a lightbulb and no box, and the cell's bracket (or border) is red

#### Scenario: An error with no explanation

- **WHEN** a cell's output is an error with no explanation, such as `frobnicate: something went wrong.`
- **THEN** Maxima's text shows in monospace in the normal text colour after the error icon, with no pink background and no red text

#### Scenario: Maxima's own text

- **WHEN** the user clicks "Show Maxima's error" on an explained error
- **THEN** Maxima's text shows in monospace under the explanation, and the button says "Hide Maxima's error"

### Requirement: Printed output reads as output

Text a cell printed (with `print`, `display` or similar) SHALL be shown in the primary text colour, in monospace, as the cell's result is shown at full contrast.

#### Scenario: print

- **WHEN** a cell runs `print("n =", 3)$`
- **THEN** "n = 3" is in the primary text colour, not the secondary one

### Requirement: Notes mark code and stay quiet

A note SHALL start with an info icon from the app's icon set, set the code its template marks in the code font, and offer Maxima's own text through a button without an underline. Explanations' and notes' templates SHALL mark code (function names, expressions, values) by enclosing it in backticks, translations SHALL keep them, and the window SHALL show each enclosed span in the code font without the backticks. A parameter's value SHALL appear as it is, even when it contains a backtick. MCP results SHALL carry the same text with the backticks.

#### Scenario: rat's replacements

- **WHEN** `rat(0.5*x)` prints its replacement and the window shows the `note.rat_replaced` note
- **THEN** "0.5 → 1/2" is in the code font and no backtick is shown

#### Scenario: A value with a backquote

- **WHEN** a note's parameter is an expression containing a backquote
- **THEN** the note shows the expression with its backquote, and the rest of the note keeps its formatting

#### Scenario: Over MCP

- **WHEN** an MCP client runs a cell that gives the `note.rat_replaced` note
- **THEN** the note's `text` reads "Maxima replaced decimals with exact fractions: `0.5 → 1/2`." with backticks

### Requirement: Suggestions sit quietly under the output

When suggestions are shown, they SHALL appear as one row of small text buttons, each starting with an arrow, lined up with the cell's output and inside the cell, with no background band. The setting that hides suggestions SHALL keep hiding them.

#### Scenario: After a result

- **WHEN** a cell's result gets the suggestions Simplify, Integrate and Plot
- **THEN** they show as "→ Simplify", "→ Integrate" and "→ Plot" under the output, starting where the output starts, with no band wider than the cell

### Requirement: A cell shows whether it's running or queued

A running cell's label SHALL read In [\*] and its bracket (or border) SHALL be in the accent colour. A cell waiting in a Run All queue SHALL read In [·], with its bracket (or border) drawn as for an idle cell. A cell that has run SHALL read In [n] as now. When a Run All starts, every cell with code it will run SHALL become queued (a blank cell doesn't); when the run stops early, each cell still queued SHALL get back the status it had before the run. MCP clients SHALL see the same `queued` status in a cell's state during the run.

#### Scenario: Run All part way

- **WHEN** Run All is running the second of four cells
- **THEN** the second reads In [\*] with an accent bracket, the third and fourth read In [·] with plain brackets, and the first reads In [1]

#### Scenario: A run stopped early

- **WHEN** Run All stops at its first cell (a trust check, an error or a cancel), and the second cell had run before
- **THEN** the second cell is no longer queued and shows its earlier result and status, and a cell that had never run reads In again

### Requirement: Silent statements leave no placeholder

A cell whose last statement ends in `$` (ignoring trailing whitespace and comments) and that produced no output SHALL show nothing under its input. A cell that ends in `;` and produced nothing SHALL show "(no output)".

#### Scenario: A definition

- **WHEN** a cell `f(x) := x^2 - 2$` has run
- **THEN** no output area shows under it

#### Scenario: A comment after the dollar

- **WHEN** a cell `g(x) := x$ /* helper */` has run
- **THEN** no output area shows under it

#### Scenario: Nothing to show

- **WHEN** a cell ending in `;` ran and gave no result, text or plot
- **THEN** it shows "(no output)"

### Requirement: Both cell styles show the same output

Every requirement above SHALL hold in bracket style and in card style, the only difference being whether the bracket or the card's border carries the red or accent state.

#### Scenario: The same error in both styles

- **WHEN** the same failed cell is shown in bracket style and in card style
- **THEN** its title, icon, explanation, suggestion and button look the same, and the red is on the bracket in one and the border in the other
