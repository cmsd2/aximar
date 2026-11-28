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
- **A cell's output, in both styles** (the `notebook-look` change; items
  8 to 11 and 22):
  - Printed text is in the primary text colour (item 9).  In card style
    the output well stays grey (item 8); bracket style has no well.
  - A failed cell shows red once: its bracket or border.  The title is
    in the normal text colour after a small red icon, the explanation is
    secondary text, and the suggestion has a lightbulb and no box.  An
    error Aximar can't explain shows Maxima's text in neutral monospace
    under the same icon.  "Show raw error" reads "Show Maxima's error"
    (item 10).
  - Notes have the Tabler info icon, code in the code font, and a quiet
    "Show Maxima's text" button right after their text, so a wrapped
    note keeps it beside the words (item 11).  The code comes from
    backticks in the catalog's templates, which translators keep and MCP
    results carry.
  - A running cell reads In [\*] with an accent bracket or border, and a
    queued one reads In [·] (item 22).  A definition ending in `$` shows
    no "(no output)".
  - Nothing sits under a result for its suggestions.  The gap below it
    has one + (both styles now draw gaps the same way), whose menu
    offers Code, Markdown and, under "From Out [n]", the result's
    next steps ("→ Simplify"); Tab at the caret opens it.  Saving a
    plot moved to the output's hover toolbar (SVG, PNG, JSON).  The
    plot viewer's export buttons use the shared `Button`.

- **Display maths in Markdown** (item 20).  `$$…$$` written on one line
  renders as display maths, as in Jupyter; before, only the form with the
  dollars on lines of their own did (`lib/remark-display-math.ts`).

## Menus and toolbars

Aximar's commands live in six places, each built on its own: the native
menu bar, the toolbar, the tab bar, buttons that appear on a hovered cell,
keyboard shortcuts, and the command palette.  No one place lists them all,
and many commands exist in only one.

| Command | Menu bar | Toolbar | Elsewhere |
| --- | --- | --- | --- |
| New, Open, Save, Save As, Export, Print | yes | | tab bar + (New) |
| Undo, Redo | Edit (the text field's own) | yes | Cmd+Z, Cmd+Shift+Z, Cmd+Y |
| Find, Find and Replace | | | Cmd+F, Cmd+Shift+F only |
| New code or Markdown cell | | yes | the gap's + and caret |
| Run cell, run and stay | | | Shift+Enter, Cmd+Enter, hover ▶ |
| Run All, Restart | | yes | |
| Interrupt | | | hover stop button on a running cell |
| Delete, move a cell | | | Cmd+D, Cmd+Shift+Up/Down, hover ▲ ▼ × |
| Variables, Log, Docs panels | | yes | |
| Templates | | yes | |
| Settings | Aximar (macOS), Edit (elsewhere) | yes | Cmd+, |
| Palette | | | Cmd+K |
| Close tab, move tab to a window | | | Cmd+W, the tab's context menu |

25. **The menu bar holds only the file commands.**  It has Aximar, File,
    Edit and Window, and Window has only Minimize.  There is no View,
    Cell, Session or Help menu.  On macOS the menu bar is where people
    look for a command and learn its shortcut, so Run All, Restart,
    Interrupt, Find and the panels can't be found there.  The shortcut
    hints shown while Cmd is held make up for some of this.
26. **Edit's Undo and Redo are the text field's, not the notebook's.**
    They are the predefined items, which act on the focused text.  The
    notebook's undo (cells added, deleted, moved) runs only from the
    toolbar and from Cmd+Z caught in the window, so the menu item and the
    shortcut beside it can do different things.
27. **Shortcuts collide or repeat.**  Cmd+W is both the tab bar's Close
    Tab and the menu's Close Window; which wins depends on the platform
    and needs checking in the app.  Cmd+N (New Notebook) and Cmd+T (new
    tab) do the same thing.  Cmd+P prints, where many editors open a
    finder.  Cmd+D deletes a cell with no confirmation, one key from Cmd+S
    and Cmd+F.  Every label says Cmd, on Windows and Linux too.
28. **The toolbar puts five kinds of control in one row at one weight:**
    inserting (+ Cell, + Markdown), running (Run All, Restart), editing
    (Undo, Redo), opening things (Templates, Settings) and panel toggles
    (Variables, Log, Docs), then the notebook's name and the session
    status.  Undo, Redo and Settings repeat the menu bar; the name
    repeats the tab; Templates is a kind of New.  Two panels open make
    two solid accent buttons, the strongest things in the window, for
    state that the open panels already show.  Restart sits far from the
    status it changes.
29. **Three icon styles.**  The tab bar's + is a character, the gap's +
    is a Tabler icon, the hovered cell's ▲ ▼ ▶ × are characters in boxes,
    the find bar has Aa ▲ ▼ ↔ ×, the docs panel ← →, and the log, docs and
    variables each draw their own ×.
30. **The palette searches functions and symbols, not commands.**  Cmd+K
    opens a function and symbol finder called "Palette"; it can't run
    Run All or open a panel, which is what a command palette does
    elsewhere.
31. **Panels have different headers.**  Variables has an upper-case title
    and Kill All and no close button; Log has tabs, Clear and a ×; Docs
    has back and forward, a search field and a ×; the find bar has its own
    rows of buttons.  Heights, close buttons and button styles differ.
32. **Only tabs have a context menu.**  A right-click on a cell, a result
    or a panel does nothing of Aximar's own.
33. **The session's state shows in three places:** the tab's dot, the
    toolbar's pill and the status bar's last log line.

*Done in the menus-and-commands change*
(`openspec/changes/menus-and-commands`): one list of the window's
commands (`src/commands/registry.ts`) builds the menu bar (File, Edit,
Cell, Session, View, Window, Help, every command with its shortcut), the
palette's Commands group, the cell context menu, the toolbar's tooltips,
the shortcut hints and Help > Keyboard Shortcuts (items 25, 30, 32).
Edit's Undo and Redo undo the notebook, or a text field's own typing
(26). Each shortcut is bound once, as a menu accelerator: Cmd+W closes
the tab, Cmd+Shift+W the window, Cmd+T is gone, and Delete Cell moved
from Cmd+D to Cmd+Shift+Backspace; labels follow the platform (27). The
toolbar holds Run All, Interrupt, Restart and the status, then the panel
toggles, tinted when on, and Settings (28, 33's pill stays beside the
session's buttons). Tabler icons replace the characters, and the
variables, docs and log panels and the find bar share one header (29,
31). Still to check by hand: the accelerators on Windows and Linux.
On Windows no accelerator fired while the page had the keyboard, which
is nearly always: WebView2 takes key presses in its own process, and
neither wry nor Tauri passes them to the menu. So on Windows the page
runs the menu's shortcuts itself (`useMenuBar.ts`), in the capture
phase, before the cell editor sees them, as the menu takes them first on
macOS. Should the menu also report one, its event is ignored. Linux,
where GTK's accelerators get the key first, is still to check.

### Proposal

- **The menu bar lists every command, with its shortcut.**
  - File: New Notebook (Cmd+N), New from Template..., Open..., Save,
    Save As..., Export as LaTeX..., Print..., Close Tab (Cmd+W), Close
    Window (Cmd+Shift+W).
  - Edit: Undo and Redo wired to the notebook's undo, Cut, Copy, Paste,
    Select All, Find (Cmd+F), Find and Replace (Cmd+Option+F).
  - Cell: New Code Cell, New Markdown Cell, Run (Shift+Enter), Run and
    Stay (Cmd+Enter), Run All, Move Up, Move Down, Delete Cell.
  - Session: Interrupt, Restart.
  - View: Variables, Log and Docs with check marks, Command Palette,
    Shortcut Hints.
  - Window: Minimize, Zoom, Next and Previous Tab, Move Tab to New
    Window.
  - Help: Aximar Help (the docs panel), Keyboard Shortcuts, What's New,
    Report an Issue.
- **A smaller toolbar in three groups:** the session (Run All,
  Interrupt, Restart and the status beside them), then space, then the
  panel toggles as one row of icons with tooltips (Variables, Docs,
  Log with its count) and Settings.  Insert moves to the gaps, undo to
  the menu and Cmd+Z, Templates to File and the empty notebook, the
  name to the tab.
- **Tabler icons for every icon button**, through `ui/icons.ts`, with
  one close button and one header component for the panels.
- **Commands in the palette,** from the same list the menu bar is built
  from, so the two can't drift apart.
- **A context menu on cells** with the Cell menu's commands.
- **Shortcut labels that follow the platform:** Cmd on macOS, Ctrl
  elsewhere.

## Suggested order

1. Quick fixes 1 to 4.
2. Stories for the screens above, so every later change shows in the
   gallery.  *Done;* see items 18 to 24 for what they turned up.
3. Tokens: bundled fonts, the type and spacing scales, one palette with
   a light and a dark value for each colour.  Then move the rules onto
   them.
4. The notebook: one output style, quieter errors, notes with code
   formatting.  *Done;* see "Changed since".
5. The toolbar, an icon set and the capitalisation rule: see "Menus and
   toolbars" above.
