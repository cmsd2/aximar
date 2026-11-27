## Why

A cell's output is the reason the notebook exists, but its parts look like they came from different apps. The audit (`docs/ui-audit.md`, items 8 to 11 and 22) and screenshots of bracket style show:

- a failed cell signals the failure three times (a red bracket or border, a red title, and a pink box with a red bar), and an error Aximar can't explain is red monospace in a pink box;
- the suggestion chips sit in a grey band the full width of the notebook, the heaviest thing on the page;
- printed text is grey, so a cell's own output looks disabled;
- notes use a ⓘ character, set function names in the prose font, and underline their link;
- a running cell shows only a small `*`, and a queued cell looks like one that will never run;
- a definition ending in `$`, which is silent by design, shows "(no output)".

Bracket style is the layout to keep; these changes are to what sits inside a cell, in both styles.

## What Changes

- **One signal for an error.** The bracket (bracket style) or the border (card style) stays red; the title is in the normal text colour with a small red error icon; the explanation is in secondary text; the suggestion has a lightbulb icon and no box. "Show raw error" becomes "Show Maxima's error". An error with no explanation shows Maxima's text in neutral monospace under the same icon.
- **Quiet suggestions.** The chips become a row of small text buttons ("→ Simplify") lined up with the output, inside the cell's output area, with no band. The setting that hides them stays.
- **Printed output at full contrast**, in the primary text colour.
- **Notes** get an icon from the app's icon set, code in the code font, and a quiet link to Maxima's own text. Explanations' and notes' templates in the catalog mark code with backticks, which translators see and keep; the window shows those spans as code, and MCP results keep the backticks.
- **Run states:** a running cell's label is In [\*] with the bracket or border in the accent colour; a queued cell's is In [·] with a dashed bracket or border.
- **No "(no output)" for silent statements.** A cell whose last statement ends in `$` shows nothing under it; a cell that ends in `;` and produced nothing still says "(no output)".
- **Storybook** gets a toolbar switch between card and bracket style, so every cell story can be checked in both.

## Non-goals

- Changing bracket style's layout (brackets, labels, output on the page) or card style's cards.
- Which suggestions the core offers, or how errors are recognised and explained: only how they look.
- The output well's grey in card style (audit item 8) beyond what the items above change; card style stays secondary.
- One palette for both themes (audit item 6), and the toolbar, menus and icons outside cells, which `menus-and-commands` (cmsd2/aximar#124) covers.
- The VS Code extension's rendering of notes and errors; it gets the backtick markup as text and can follow later.

## Capabilities

### New Capabilities

- `cell-output`: how a code cell's output and state look: errors, notes, printed text, suggestions, run states, and empty output, in both cell styles.

### Modified Capabilities

None. The message templates' code markup is a convention of the catalog's text, covered under `cell-output`; the `maxima-messages` spec in the messages-and-localisation change, still unarchived, describes templates and IDs, which stay as they are.

## Impact

- **Frontend:** `CellOutput.tsx`, `EnhancedErrorOutput.tsx`, `ErrorOutput.tsx`, `TextWithNotes.tsx`, `CellSuggestions.tsx`, `Cell.tsx` (labels and states), `lib/messages.ts` (code spans), `ui/icons.ts` (error and lightbulb icons), `global.css`, stories for every state in both styles, and `.storybook/preview.tsx` (the style switch).
- **Core:** the English templates of the explanations and notes that name code gain backticks (`crates/aximar-core`, `docs/maxima-messages.md`); IDs and parameters are unchanged. Catalogs are regenerated, so existing translations of those entries need updating.
- **MCP:** `text` fields of notes and explanations now carry backticks around code; nothing else changes.
- **Tauri, tools:** none.
- **Related:** cmsd2/aximar#44 (the teaching pilot, which asked for the suggestions setting already shipped); cmsd2/aximar#124 also edits `Cell.tsx`'s actions and `ui/icons.ts`, so whichever lands second rebases.
