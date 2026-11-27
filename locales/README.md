# Translating Aximar

Each language is one file here, `<lang>.po`, in gettext's format: the same
format as Maxima's own translations. `messages.pot` is the template the
files are made from; don't edit it, or `en.po` and `pseudo.po`, by hand.

## What you translate

- **The window and the menus**: buttons, settings, dialogs, menu items.
- **Aximar's explanations of Maxima's output**: error explanations (their
  context starts `error.`), notes (`note.`) and questions (`question.`).
  [docs/maxima-messages.md](../docs/maxima-messages.md) lists them, with
  the Maxima text each one explains.

What you don't translate here:

- **Maxima's own messages**, such as `expt: undefined: 0 to a negative
  exponent.` Aximar shows Maxima's own translation of them beside its
  explanation. Fixes and missing translations go to Maxima, in
  `locale/<lang>.po` of Maxima's repository.
- **Results, print output and code**: Aximar never translates them.

## Working on a file

1. Open `<lang>.po` in [Poedit](https://poedit.net/), Lokalize or OmegaT.
   Each entry shows the English, a note saying where the text appears and
   what its placeholders hold, its context (the message's ID, for
   explanations) and the source file.
2. Load `glossary.tbx` as the glossary (OmegaT, Lokalize), or keep it open
   beside you.
3. To see your work in Aximar, use a pre-release build: Settings, "Load
   translation file...", and choose your `.po`. Each save updates the
   window and the menus.
4. Check the file before sending it:
   `msgfmt --check --output-file=/dev/null <lang>.po`
5. Send the file back, or open a pull request. CI checks placeholders and
   length limits too.

Entries marked fuzzy are suggestions: from Maxima's catalog when the
language was added, or a translation whose English has changed since (the
old English is kept as `#|`). Aximar shows the English until you review a
fuzzy entry and clear the mark.

## Rules

- **Placeholders** in braces, such as `{func}` or `{count}`, stay exactly as
  they are, spelt the same; move them where your language needs them. A
  translation that drops or renames one fails CI.
- **No apostrophe next to a brace**: `'{name}'` hides the value. Use
  double quotes, or none.
- **Length limits**: a note with `max-length:N` means the text has to fit
  in N characters, such as a menu item or a toolbar button. CI checks it.
- **Plurals**: entries with plural forms have one translation per form;
  Poedit shows a tab for each.
- **Keep in English**: Maxima, Aximar, LaTeX, TeX, MCP, Docker, WSL,
  Plotly, Claude Code, Codex, Gemini CLI, and any Maxima function, option
  or variable name (`assume`, `integrate`, `ratprint`). Code in an entry,
  such as `assume(a > 0)$` or `load("draw")$`, stays exactly as it is,
  including `.` as the decimal point.
- **Capitalisation**: the English uses title case for titles ("Division by
  Zero"). Use your language's own rule; Spanish uses sentence case
  ("División por cero").

## Spanish

- **Tú or usted**: Maxima's Spanish uses both. Choose one with the
  reviewer, note it here, and use it throughout.
- **fichero or archivo**: Maxima says fichero, as in Spain; archivo is
  usual in Latin America. Choose one and note it here.
- `glossary.tbx` follows Maxima's own terms where Maxima has them
  (argumento, función, matriz, no definido), so Aximar's explanations
  match the Maxima messages shown beside them.

## Adding a language

`scripts/i18n-add-language <lang>` makes `<lang>.po` from the template,
with Maxima's catalog for the language (if it has one) suggesting
translations. Then add the language to `locales` in `lingui.config.ts`.
See [docs/i18n.md](../docs/i18n.md) for how the catalogs fit together.
