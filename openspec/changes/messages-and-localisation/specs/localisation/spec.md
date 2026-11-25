## Purpose

How Aximar shows its text, Maxima's messages and the help in the user's language: the language setting, falling back to English one message at a time, Maxima kept in English underneath, and the checks and tools that keep translations correct.

## ADDED Requirements

### Requirement: Maxima runs in English

Aximar SHALL start Maxima so that its messages are in English whatever the user's environment, on the local, Docker and WSL backends, and SHALL recognise failed evaluations the same way in every environment.

#### Scenario: Spanish environment (needs Maxima)

- **WHEN** Aximar starts a local session with `LANGUAGE`, `LC_ALL`, `LC_MESSAGES` and `LANG` all set to `es_ES.UTF-8`
- **THEN** `1/0;` SHALL be an error, `%` SHALL still hold the result before it, and a syntax error in debug mode SHALL return instead of waiting for input

### Requirement: The user chooses the language

Aximar SHALL have a language setting, listing English and each language with a catalog. Until the user chooses, it SHALL use the system language when Aximar has a catalog for it (matching `es_CO` to `es`), and English otherwise. Aximar SHALL save the choice.

#### Scenario: System language with a catalog

- **WHEN** Aximar starts for the first time on a system whose language is `es_CO`
- **THEN** the language setting SHALL be Spanish

#### Scenario: System language without a catalog

- **WHEN** Aximar starts for the first time on a system whose language is `ja`, and there's no Japanese catalog
- **THEN** the language setting SHALL be English

### Requirement: Switching language takes effect without a restart

Changing the language SHALL change the window's text, the native menus and the help summaries at once, and Maxima's translated errors from the next evaluation, without restarting Aximar or the Maxima session. Cells already run SHALL show their explanations and notes in the new language.

#### Scenario: Switch to Spanish

- **WHEN** the user changes the language from English to Spanish with a notebook open and a session running
- **THEN** the toolbar, the menus and an existing cell's error explanation SHALL show Spanish text from the catalog, and the session SHALL keep its variables

#### Scenario: Maxima's errors follow the switch (needs Maxima)

- **WHEN** the user switches to Spanish and then runs `diff();`
- **THEN** the cell SHALL show `diff: número de argumentos equivocados.`

### Requirement: Missing translations fall back to English

A message without a translation in the user's language SHALL show in English, without affecting the messages around it. A key or template SHALL never show in place of text.

#### Scenario: Partly translated catalog

- **WHEN** the Spanish catalog translates the toolbar's "Run all" but not the "Show original" button
- **THEN** the window SHALL show the toolbar in Spanish and the button in English

### Requirement: Maxima's errors show in the user's language

When a cell fails with a Maxima error that Maxima's own catalog translates into the user's language, the cell SHALL show that translation, with the parameters Maxima formatted, and SHALL keep Maxima's English text one click away. This SHALL hold at the top level, inside `errcatch` and in the debugger. A cell whose error `errcatch` caught SHALL succeed without a translation. A message Maxima doesn't translate, or a Maxima without catalogs, SHALL show the English alone.

#### Scenario: Translated error (needs Maxima)

- **WHEN** the language is Spanish and a cell runs `1/0;`
- **THEN** the cell SHALL be an error, its translation SHALL be `expt: no definido: desde 0 a un exponente negativo.`, and its English text SHALL be Maxima's English message

#### Scenario: Untranslated error (needs Maxima)

- **WHEN** the language is Spanish and a cell runs `error("my own message");`
- **THEN** the cell SHALL be an error with no translation

#### Scenario: English (needs Maxima)

- **WHEN** the language is English and a cell runs `1/0;`
- **THEN** the cell SHALL have no translation

### Requirement: Computations are never translated

Results, `print` output, function names, option names and Maxima's code SHALL show as Maxima gives them in every language. Notebook files SHALL hold the same content whichever language saved them.

#### Scenario: Result and print output

- **WHEN** the language is Spanish and a cell runs `print("hello")$ x^2/2;`
- **THEN** the cell SHALL show `hello` and `x^2/2` unchanged

#### Scenario: Same file in any language (needs Maxima)

- **WHEN** the same notebook runs and is saved once in English and once in Spanish
- **THEN** the two files SHALL be identical apart from timings

### Requirement: MCP stays in English

MCP results SHALL carry English text, with message IDs and parameters, whatever the language setting.

#### Scenario: Connected mode in Spanish (needs Maxima)

- **WHEN** the language is Spanish and an MCP client runs a cell that fails with `1/0;`
- **THEN** the result's `error_info` SHALL have English text and `id` `error.division_by_zero`

### Requirement: Help summaries in the user's language

Help summaries SHALL show in the user's language when Maxima's manual for that language covers the symbol, and in English otherwise. Search SHALL find symbols by their summaries in either language.

#### Scenario: Spanish summary

- **WHEN** the language is Spanish and the user hovers `integrate`
- **THEN** the summary SHALL be the one from Maxima's Spanish manual

#### Scenario: English fallback

- **WHEN** the language is Spanish and the user hovers a symbol the Spanish manual lacks
- **THEN** the summary SHALL be the English one

### Requirement: Translators can preview their work

Pre-release builds SHALL let the user load a translation file from disk for any language, and SHALL apply it again each time the file changes, without a restart. They SHALL offer a mode in which pointing at any translatable text shows its source text, context, translator note and source location. Release builds SHALL offer neither.

#### Scenario: Live preview

- **WHEN** a translator loads `es.po` in a pre-release build, changes the translation of "Run all" in an editor and saves
- **THEN** the toolbar SHALL show the new text

#### Scenario: Show sources

- **WHEN** show-sources mode is on and the user points at the "Show original" button
- **THEN** Aximar SHALL show its source text, its translator note and the file it comes from

### Requirement: The catalog is checked

The build SHALL fail when the catalog's template is out of date with the source, when a translation file isn't valid, when a translation drops or renames a placeholder, or when a translation exceeds a length limit its entry declares.

#### Scenario: New text without extraction

- **WHEN** a change adds window text without updating the template
- **THEN** CI SHALL fail, naming the missing string

#### Scenario: Broken placeholder

- **WHEN** a translation renders `{fn} expects {n} arguments` as `{fn} espera {m} argumentos`
- **THEN** CI SHALL fail, naming the entry and the placeholder
