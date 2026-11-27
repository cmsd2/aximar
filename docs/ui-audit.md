# UI audit, November 2026

What stands between Aximar and a professional look, ranked by how much
each item shows and how cheap it is to fix.  Taken from screenshots of
every Storybook story in light and dark at 1x and 2x, contrast measured
in the browser, and a count over `src/styles/global.css`.  The live app
wasn't checked; see "Screens with no story" for what that leaves out.

## Quick fixes

These are bugs more than design work.  All four are fixed: the export
dialog has a primary button and fits its content, every text in the
stories reaches 4.5:1 in both themes (the Settings window's Claude status
aside, which the settings-window change replaces), `--font-mono` holds the
one monospace stack, and the Shortcuts story shows the hints.  The
trust banner, syntax colours and warning colours are theme variables now.

1. **The Export as LaTeX dialog is unstyled.**  Its Export button uses
   the class `template-start`, which has no CSS, so it shows as a bare
   browser button.  The dialog borrows `settings-modal`, so it takes the
   Settings window's fixed height and leaves about 500px of empty white
   below two checkboxes.  The "Exporting entire notebook" line sits 20px
   further in than the options under it.
2. **Text below WCAG AA contrast** (4.5:1 for body text):

   | Where | Theme | Ratio |
   | --- | --- | --- |
   | Trust banner text | light | 1.7:1 |
   | Trust Notebook button | light | 1.8:1 |
   | "Show raw error" | light | 1.9:1 |
   | "Show raw error" | dark | 3.1:1 |
   | Error title | light | 3.8:1 |
   | One code token colour (`ͼt`) | dark | 3.0:1 |
   | Note text on the output background | light | 4.35:1 |

3. **`--font-mono` is never defined.**  The slider and number controls
   of reactive cells use `var(--font-mono, monospace)`, so they get the
   browser's default monospace font.
4. **The Shortcuts story is blank.**  The hints show only while Cmd is
   held, and the story doesn't hold it, so the gallery has nothing to
   show for them.

## The system underneath

5. **Fonts depend on the machine.**  No font ships with the app.  The
   monospace stack starts with JetBrains Mono, which most people don't
   have, so code and output fall back to SF Mono or Menlo on a Mac and to
   Consolas on Windows.  Three different monospace stacks appear in the
   stylesheet.  Bundling one monospace font (JetBrains Mono is under the
   OFL) and defining `--font-mono` and `--font-ui` once fixes both.
   *Done:* the design-library change defines `--font-ui` and
   `--font-mono` once, in `src/styles/tokens.css`, and the app bundles
   JetBrains Mono, with its licence in `public/licenses/`.
6. **The two themes come from two design languages.**  Light uses Apple's
   system colours (`#0071e3`, `#1d1d1f`, `#f5f5f7`).  Dark uses Catppuccin
   Mocha's pastels (`#89b4fa`, `#f38ba8`, `#a6e3a1`).  So the accent,
   error and success colours change hue between themes, not only
   lightness, and the same screen feels like a different app in each.
   Eleven colours sit in rules as raw hex instead of tokens.
7. **No scales.**  The stylesheet is 3,233 lines, with 9 font sizes (10
   to 20px), 59 distinct padding values and 8 corner radii (3, 4, 6, 8,
   10 and 12px, 50%, 0).  The Out label needed three fixes this week
   because two neighbouring parts used different paddings and line
   heights.  A type scale of 4 or 5 steps, spacing on a 4px grid and two
   or three radii, all as variables, removes that class of problem.
   *Done for new UI:* `src/styles/tokens.css` holds the scales, the
   Settings window and Export LaTeX use them, and `npm run check:tokens`
   holds the rest of the stylesheet to a baseline that only falls.  See
   [design-library.md](design-library.md).

## The notebook

8. **The output area reads as disabled.**  Output sits in a grey well
   darker than the input above it, the opposite of the usual hierarchy,
   where the result is the thing you came for.
9. **One cell mixes several unrelated styles.**  Printed text is grey
   monospace, maths is KaTeX's serif, notes are the system sans with a
   ⓘ character, and an explained error is a red card.  An error with no
   explanation looks different again: pink monospace in a dark box.
10. **Errors shout.**  A failed cell gets a red border, a red title, and
    a red-barred pink suggestion box, three signals for one fact.
11. **Notes set code in the prose font.**  `ratprint: false$`,
    `quad_qags` and `solve` appear as plain words, and a note that starts
    with a function name starts with a lowercase letter.  On a note that
    wraps, "Show Maxima's text" moves to the far right edge.
12. *Withdrawn:* the "In" label on a Markdown cell came from the stories,
    which drew every cell as a code cell.  They now render the
    `Notebook` component, which gives Markdown cells their own view.

## The window

13. **The toolbar is a row of 13 text controls.**  The notebook title
    ("Untitled *") sits between Docs and Settings with no separator.  The
    Variables toggle, when on, is a filled accent button, which makes it
    the strongest thing on the screen.  The Log badge overlaps the word
    Log.  The session status is a red "Stopped" pill; if that is also
    the state before the first run, it reads as an error.
14. **Text characters stand in for icons**: ×, ⓘ, ⚠, ▸ and ▾.  Their
    size and weight follow the font, so they never line up with each
    other or with the text beside them.
    *Done for new UI:* Tabler is the icon set, imported through
    `src/components/ui/icons.ts`; the older screens still use the
    characters until they move onto it.
15. **Capitalisation is mixed.**  "Starter Notebooks", "Trust Notebook",
    "Delete Cell", "Find & Replace" and "Division by Zero" are in title
    case.  "Start with empty notebook", "Linear algebra" and "Show raw
    error" are in sentence case.  A rule such as title case for buttons
    and menus and sentence case for everything else, applied once, would
    also help the translators.
16. **Dialogs have fixed heights.**  Templates and the command palette
    keep a tall empty area when the list is short.  In Templates the
    cards sit 8px from the edge while the heading sits 20px in.
17. **Settings** wraps its segmented labels ("Sans-serif", "Computer
    Modern") onto two lines.  The settings-window change replaces this
    dialog, so it needs nothing more here.

## Screens that now have stories

The docs panel, the log window and status bar, Find and Replace,
completion, argument help, SVG and Plotly plots, reactive controls, the
plot viewer, rendered Markdown, running and queued cells, the whole
window, first run and Maxima not found all have stories now (`Panels`,
`Editor`, `Outputs` and `Screens` in Storybook).  They show these
problems:

18. **A missing Maxima squeezes the toolbar.**  The status pill holds the
    whole error ("Error: Failed to start Maxima: maxima: No such file or
    directory (os error 2)"), so "+ Cell", "+ Markdown" and "Run All"
    wrap onto two lines and the notebook title shrinks to "U…".  The
    window gives no way forward apart from a native dialog that shows
    once.  The pill should say "Maxima not found" and link to the Maxima
    settings, as the log's entries already can.
19. **Times break on a 12-hour clock.**  The log's time column wraps "AM"
    onto a second line, and in the Events tab the time runs into the
    event's name.  The status bar's unread badge covers the "M" of "PM".
20. **Markdown display maths renders inline.**  `$$…$$` on one line comes
    out as inline maths in a Markdown cell.  The docs panel already
    converts that form (`preprocessDocsMath`); cells don't.  Links in
    Markdown cells keep the browser's default blue and underline, which
    measures 1.75:1 in dark.
21. **Find marks only the current match.**  "2 of 8" highlights one `x`,
    and the other seven don't show.
22. **A queued cell looks idle.**  A queued cell has the same plain "In"
    label and border as a cell that will never run.  The running cell
    has only a small dot.
23. **gnuplot plots stay white in dark.**  A plot2d SVG is a white
    rectangle inside the dark output well.
24. **More contrast below 4.5:1:** accent text on the secondary
    background (docs signatures and links, the log's settings link and
    event names) at 4.31:1 in light; Markdown table cells at 4.26:1; the
    variables panel's × at 2.21:1 in light and 3.49:1 in dark; the status
    bar badge at 2.32:1 in dark.

## Changed since

- **Selecting cells in card style.**  The bracket right of each card is
  gone.  The column with a cell's In and Out labels selects it (a
  Markdown cell's indent does the same), and a selected card gets an
  accent ring outside its border, so a running or error border still
  shows.  A plain click selects one cell, Cmd or Ctrl adds or removes
  one, and Shift selects a range; bracket style uses the same clicks on
  its bracket.  The 48px right margin the bracket needed is gone too.
- **Inserting cells in card style.**  The + Code and + Markdown bar
  needed more room than the 8px between two cards.  Now hovering a gap
  shows a line and a + in the left margin, whose menu inserts a code or
  Markdown cell.  Clicking a gap places a caret there: the stretch of
  line under the In and Out labels turns blue and blinks.  Typing
  starts a code cell with that text, # starts a Markdown cell,
  Enter starts an empty code cell, and Escape removes the caret.  Bracket
  style keeps its bar, and the caret shows in it too.
- **The arrow keys cross cells.**  From a cell's first or last line, Up
  or Down goes to the caret in the gap beside it, and from there into
  the next cell (a raw cell is skipped).  Shift+Enter on the last cell
  puts the caret below it instead of adding an empty cell, so a new cell
  appears only once you type.  New cells take the cursor, from the gap
  or the toolbar; before, the toolbar's new cell didn't.
- **New notebooks start empty, with the caret.**  The tab bar's + and
  File > New open a notebook with no cells and the caret ready, so
  typing starts the first cell.  A click in the space below the last
  cell (30% of the window's height, and all of an empty notebook) places
  the caret after it.  Notebooks from MCP, templates and files still
  have at least one cell.

- **Suggestion chips can be turned off**, in Settings > Notebook, for
  a class where students should write each step themselves
  (cmsd2/aximar#44).  The chips also keep their 8px under a card, which
  the card spacing change above had taken away.

## Suggested order

1. Quick fixes 1 to 4.
2. Stories for the screens above, so every later change shows in the
   gallery.  *Done;* see items 18 to 24 for what they turned up.
3. Tokens: bundled fonts, the type and spacing scales, one palette with
   a light and a dark value for each colour.  Then move the rules onto
   them.
4. The notebook: one output style, quieter errors, notes with code
   formatting.
5. The toolbar, an icon set and the capitalisation rule.
