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
   *Part done:* the design-library change defines `--font-ui` and
   `--font-mono` once, in `src/styles/tokens.css`; bundling JetBrains
   Mono is its group 4.
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
12. **The markdown cell's editor shows an "In" label** like a code cell.

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

## Screens with no story

The audit couldn't see these.  Each needs a story before it can be
judged or kept in shape:

- the docs panel and the log panel
- the find bar and the completion and argument-help popups
- plots, both SVG and Plotly, and the plot viewer
- rendered markdown cells
- reactive controls (sliders and number fields)
- cells running and queued, and the whole window together
- first run, and Maxima not found

## Suggested order

1. Quick fixes 1 to 4.
2. Stories for the screens above, so every later change shows in the
   gallery.
3. Tokens: bundled fonts, the type and spacing scales, one palette with
   a light and a dark value for each colour.  Then move the rules onto
   them.
4. The notebook: one output style, quieter errors, notes with code
   formatting.
5. The toolbar, an icon set and the capitalisation rule.
