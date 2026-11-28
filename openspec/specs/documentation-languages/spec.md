# documentation-languages Specification

## Purpose
Documentation for Maxima's symbols can exist in several languages. Front ends show it in the reader's language where it exists, fall back to English where it doesn't, and say which they are showing, so a reader is never misled about what they are reading.

## Requirements

### Requirement: A doc index names its language

A doc index SHALL carry a `language` field holding a BCP 47 tag, such as `es` or `pt-BR`. An index without the field SHALL be read as English. A translated index SHALL be named `<stem>-doc-index.<lang>.json` beside its package's English `<stem>-doc-index.json`, and its `language` field SHALL match the name.

#### Scenario: An older index reads as English

- **WHEN** the catalog loads a doc index with no `language` field
- **THEN** its entries are English entries

#### Scenario: A name and field that disagree

- **WHEN** the catalog finds `maxima-core-docs-doc-index.es.json` whose `language` is `de`
- **THEN** it skips the file and logs a warning naming it

### Requirement: The catalog answers in a language chain

The catalog SHALL keep each symbol's entries for every language it loads, and SHALL answer a lookup for a chain of languages, such as Spanish then English, with the first language in the chain that has an entry for that symbol. It SHALL resolve each symbol separately, and SHALL report the language of the entry it returns. English SHALL end every chain.

#### Scenario: A translated symbol

- **WHEN** Spanish documentation for `integrate` is installed and the chain is Spanish then English
- **THEN** the lookup returns the Spanish entry, reported as `es`

#### Scenario: A symbol the translation lacks

- **WHEN** Spanish documentation is installed but has no entry for `fft`, and the chain is Spanish then English
- **THEN** the lookup returns the English entry for `fft`, reported as `en`

#### Scenario: A summary-only layer

- **WHEN** only the embedded Spanish summaries are available for `integrate`, the full English documentation is installed, and the chain is Spanish then English
- **THEN** the lookup returns the Spanish summary together with the English full documentation, each reported with its own language

### Requirement: The reader sees which language they're reading

The docs panel SHALL show a symbol in the chosen language when its documentation exists in it. When it shows English in place of the chosen language, it SHALL say so in the chosen language. When a translated entry's signatures differ from the English entry's, it SHALL say the translation may be out of date and show the English signatures.

#### Scenario: Falling back to English

- **WHEN** the chosen language is Spanish and the user opens `fft`, which has no Spanish documentation
- **THEN** the panel shows the English documentation under a note, in Spanish, saying it isn't available in Spanish

#### Scenario: An old translation

- **WHEN** the Spanish entry for a function lists one signature and the English entry lists two
- **THEN** the panel shows the Spanish text with a note that the translation may be out of date, and both English signatures

### Requirement: Search covers every installed language

Search SHALL match a query against the symbols' text in every language the chain names, and SHALL show each result in the first language of the chain that has it.

#### Scenario: A Spanish query

- **WHEN** the chain is Spanish then English and the user searches for "integral definida"
- **THEN** `integrate` is among the results, shown with its Spanish summary

### Requirement: The reader chooses the documentation language

Settings > General SHALL offer a Help language setting, beside the app's Language: "Same as Aximar's language", which is the default, and each language with documentation installed or embedded. The docs panel SHALL have no language control of its own; its notes about the help's language SHALL link to the setting. The choice SHALL persist across restarts, and every open docs panel SHALL show its page again in the new language when it changes. "Same as Aximar's language" SHALL follow the app's language setting, falling back to English when there's no documentation in it.

#### Scenario: Following the app

- **WHEN** the app's language is Spanish and the help language is "Same as Aximar's language"
- **THEN** the docs panel uses the chain Spanish then English

#### Scenario: Choosing English

- **WHEN** the app's language is Spanish and the user chooses English as the help language in Settings
- **THEN** the open docs panel shows its page again in English, and still does after a restart

#### Scenario: From a note to the setting

- **WHEN** the docs panel shows a note that a function's documentation is in another language, and the user clicks Help language settings
- **THEN** Settings opens at General, where the help language is

### Requirement: A missing language says how to get it

When the chosen language has no full documentation installed, the docs panel SHALL say so and show the `mxpm install` command for that language's core-docs pack, with a button that copies it.

#### Scenario: No Spanish pack

- **WHEN** the chosen language is Spanish and `maxima-core-docs-es` isn't installed
- **THEN** the panel shows that full Spanish documentation isn't installed and the command `mxpm install maxima-core-docs-es`, with a copy button

### Requirement: Core documentation comes in Maxima's languages

`mxpm doc generate-core-docs --lang <lang>` SHALL build the core documentation from Maxima's manual in `doc/info/<lang>`, writing `<stem>-doc-index.<lang>.json` with its `language` set. The package index SHALL list a `maxima-core-docs-<lang>` package for each language of Maxima's manual that covers at least half of the English manual's symbols, pinned to the same Maxima release as `maxima-core-docs`.

#### Scenario: Spanish core docs

- **WHEN** mxpm runs `doc generate-core-docs --lang es` on a Maxima source tree
- **THEN** it writes `maxima-core-docs-doc-index.es.json` with `"language": "es"`, holding the symbols the Spanish manual documents

### Requirement: MCP and the language server answer in a language

MCP's `get_function_docs` and `search_functions` SHALL take an optional `language`, use the chain that language then English, and name the language of each text in their results; without it they SHALL answer in English, as now. maxima-lsp SHALL use the `locale` the client sends in `initialize` for hover and completion, then English.

#### Scenario: MCP in Spanish

- **WHEN** a client calls `get_function_docs` for `integrate` with `language: "es"` and Spanish documentation is installed
- **THEN** the result holds the Spanish documentation and `"language": "es"`

#### Scenario: VS Code in Spanish

- **WHEN** VS Code starts maxima-lsp with `locale` `es` and the user hovers over `integrate`
- **THEN** the hover shows the Spanish documentation
