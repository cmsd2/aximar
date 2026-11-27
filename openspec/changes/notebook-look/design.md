## Context

A cell's output is drawn by `CellOutput.tsx`, which hands errors to `EnhancedErrorOutput.tsx` (explained) or a `<pre>` in `.error-output` (unexplained), printed text and notes to `TextWithNotes.tsx` and `RichTextOutput.tsx`, and results to `KatexOutput`. Suggestions come from `CellSuggestions.tsx`, rendered after the cell as a full-width band. `Cell.tsx` draws the In and Out labels and the cell's status class (`running`, `queued`, `error`), which bracket style's `.bracket-toggle` and card style's border pick up in `global.css`.

Notes' and explanations' text comes from the core as a template, an ID and parameters (`crates/aximar-core/src/message.rs`); the window looks the template up in the catalog (`lib/messages.ts`) and falls back to the English `text`. MCP clients get the same `text`.

The core already strips trailing comments and finds statement terminators when it prepares input (`maxima/protocol.rs`: `strip_trailing_comments`, `find_terminators`). The GUI's cell state is built from the core's notebook in `src-tauri/src/mcp/sync.rs` (`SyncCell`).

Bracket style is the layout the user keeps; card style gets the same changes inside its cards.

## Goals / Non-Goals

**Goals:**
- One place decides each look: error, note, printed text, suggestions, run state, empty output.
- The window tells a silent cell from an empty one without parsing Maxima itself.
- Code in explanations and notes is marked once, in the catalog, for every language and every client.

**Non-Goals:**
- New tokens or a new palette; this uses the design library's existing colours, spacing and Tabler icons.
- Moving output rendering into a shared package for the VS Code extension (stream S4 in `docs/project-plan.md`).

## Decisions

### Errors: the state lives on the cell, not in the output

The red goes on the bracket or border through the existing `.cell.error` class; `EnhancedErrorOutput` drops its red title colour, the suggestion's pink box and red bar, and gains a red Tabler `IconAlertCircle` before the title and `IconBulb` before the suggestion (both added to `ui/icons.ts`). The unexplained error reuses the same header: icon plus Maxima's text in a neutral `<pre>`. "Show raw error" becomes "Show Maxima's error" and "Hide Maxima's error", the wording notes already use for Maxima's text.

*Alternative:* keep a tinted background on the explanation and drop the red bracket. Rejected: the bracket is bracket style's way of showing a cell's state, and the running and queued states use it too.

### Code markup: backticks in templates, split before parameters go in

Templates mark code with backticks: "Maxima replaced decimals with exact fractions: `{replacements}`." The window renders a message by calling Lingui with each parameter replaced by a sentinel (a private-use character and the parameter's index), splitting the translated string on backticks into prose and code spans, and then putting each parameter's value back into its span. A value is never searched for backticks, so an expression containing a backquote can't break the formatting. `messageText` and `errorText` return React nodes for this; a plain-string variant stays for places that need text (tooltips, the log).

The English fallback `text` the core sends keeps the backticks, which reads as Markdown code to MCP clients and in the log.

*Alternative:* mark code only by parameters (every parameter in code font). Rejected: some parameters are prose (a count, a direction), and fixed words like `solve` in a template need marking too.

*Alternative:* HTML or Lingui rich-text tags (`<code>{x}</code>`). Rejected: tags are harder for translators to keep intact than backticks, and they'd reach MCP clients as markup.

### Silent cells: the core says so

`aximar-core` gets `ends_silent(input) -> bool` in `maxima/protocol.rs`, built on `strip_trailing_comments` and `find_terminators`: true when the last terminator outside strings and comments is `$`. `SyncCell` gains `silent: bool`, filled from the cell's current input whenever the GUI's state is built. It isn't stored: saved notebooks and MCP results don't change. The window hides the output area of a silent cell with nothing to show, and keeps "(no output)" for a non-silent one.

*Alternative:* parse the input in the window. Rejected: Maxima's strings and comments make terminators easy to misread, and the core already does it right; logic like this belongs in the core.

### Suggestions: inside the output area

`CellSuggestions` renders inside the cell's output area, after the output, as text buttons with an arrow (`→ Simplify`), indented to the output column (`--gutter-width`). The full-width band and its background go. The buttons use the design library's quiet button style in a smaller size.

### Run states: labels and the bracket

`aximar-core` gains `CellStatus::Queued`. `run_all` marks every cell with code queued when it starts, reporting the change once rather than per cell, and remembers each one's status; when it ends, any cell still queued (not reached, or stopped before it started, as by the trust check) gets that status back, again reported once. The window's types already had `queued`, which nothing set. Status isn't saved in files, so notebooks don't change; MCP clients reading a cell's status during a run can see `queued`.

`Cell.tsx` shows In [\*] for `running` and In [·] for `queued`; the labels are translatable with a note that the symbols stay. The running bracket or border uses `--accent`; a queued cell keeps an idle cell's bracket or border, as a dashed one looked busy down a long notebook. The selection bracket, which is also accent, sits apart from the cell's own bracket in bracket style and is a ring in card style, so the two don't look alike.

### Printed text

`.text-output` and the printed parts of `RichTextOutput` move from `--text-secondary` to `--text-primary`.

### Storybook

A `cellStyle` toolbar global (Card, Bracket) sets the style for stories that don't ask for one; a story's own `cellStyle` parameter still wins. Every cell story is checked in both styles, and the gallery keeps taking card style unless a story asks.

## Risks / Trade-offs

- [Editing English templates changes catalog msgids, so existing translations of those entries go fuzzy] → Spanish has few translated core messages so far; the change lists the edited IDs and the translator guide says to keep the backticks.
- [A translator drops or unbalances a backtick] → an unbalanced backtick renders the rest as prose (the split ignores a trailing odd span), and the catalog check warns when a translation's backtick count differs from the English.
- [MCP clients that show `text` raw now show backticks] → they read as Markdown code in the clients Aximar targets; noted in the CHANGELOG.
- [The accent running bracket is close to the selection bracket's colour] → they sit in different places, and the running cell also changes its label.

## Migration Plan

No data changes. The change ships with the app; rolling back restores the old look. `SyncCell.silent` defaults to false, so a window older than the core shows "(no output)" as before.
