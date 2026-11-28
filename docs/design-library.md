# Design library

Aximar's window draws its colours, sizes and spacing from tokens, and its
buttons, fields, menus, status messages and dialogs from shared parts. New
UI uses them, so it looks like the rest of the window without anyone
choosing a padding.

**The rule:** new UI uses the tokens and the parts in `src/components/ui/`,
never a raw colour, font size or corner radius, and never a text character
as an icon. CI holds the stylesheets to this (see [Checks](#checks)).

Storybook shows all of it: run `npm run storybook`, then open Foundations
for the tokens and UI for the parts. Both have a theme switch and a
language switch in the toolbar.

## Tokens

All in `src/styles/tokens.css`, and nowhere else. Variables the app sets
while it runs, such as `--font-size-mono` and `--markdown-indent`, live
in `global.css` and aren't tokens.

There are four colour palettes. Light sits in `:root`; Paper (warm
off-white with ink text) under `[data-theme="paper"]`; Dark (neutral
greys, like the system's own dark mode) under `[data-theme="dark"]` and again under
`prefers-color-scheme: dark` for Auto, which removes the `data-theme`
attribute; and Night (tinted violet, with pastel accents)
under `[data-theme="night"]`. Every colour token needs a value in each
block, and a rule that changes in dark mode is written for
`[data-theme="dark"]`, `[data-theme="night"]` and, inside the media
query, `:root:not([data-theme])`, so it leaves Light and Paper alone.

### Colour

| Job | Token |
| --- | --- |
| Body text | `--text-primary` |
| Hints, captions, quiet buttons | `--text-secondary` |
| Placeholders, disabled items | `--text-placeholder` |
| The window, dialogs, inputs | `--bg-primary`, `--bg-input` |
| Sidebars, toolbars, secondary buttons | `--bg-secondary` |
| Cells, hovers | `--bg-cell` |
| A chosen item in a list | `--selected-bg` |
| Lines and borders | `--border-color` |
| Primary buttons, links, focus rings | `--accent`, `--accent-hover` |
| Success, as text or a tint | `--success-text` on `--success-bg`; `--success-color` for marks |
| Warning, as text or a tint | `--warning-text` on `--warn-bg` |
| Errors | `--error-color`, on `--bg-primary` or `--error-bg` |
| Running | `--running-color` |
| Behind dialogs | `--overlay-bg` |
| The tab bar and toolbar under the title bar | `--chrome-bg`, `--chrome-border` |

`--chrome-bg` and `--chrome-border` are `--bg-secondary` and
`--border-color` unless the desktop gives its own. On Linux the system
draws the title bar and the GTK menu bar in the desktop theme's colours,
right above the tabs. The backend reads that menu bar's background, its
background while another window is active, and the theme's border colour
(`src-tauri/src/commands/system_chrome.rs`). `useSystemChrome` then sets
the two tokens, so the frame, menus, tabs and toolbar read as one band. It
uses the colours only when they are as light or dark as the page, and never
in Paper or Night; otherwise the tokens keep the app's own colours. Auto follows the
system's light or dark, and the backend sends new colours when the desktop
theme changes. `--font-ui` takes the desktop's interface font on Linux, such
as Noto Sans on KDE or Cantarell on GNOME. macOS and Windows send nothing.
Text on the chrome stays `--text-primary` and `--text-secondary`: on
Breeze's `#dee0e2`, `--text-secondary` is 4.5:1.

Text in `--accent` goes on `--bg-primary` or `--bg-input` only: on
`--bg-secondary` and `--bg-cell` the light accent falls below 4.5:1.
`src/styles/contrast-pairs.ts` lists every text colour with the
backgrounds it may sit on; add a pair there before putting a text colour
on a new background, and the contrast test checks it in every theme.
[Foundations / Colours](?path=/story/foundations-colours--contrast) shows
each ratio.

### Type

| Token | Size | For |
| --- | --- | --- |
| `--text-xs` | 11px | badges, counts |
| `--text-sm` | 12px | hints, captions, small buttons |
| `--text-md` | 13px | body text, buttons, fields |
| `--text-lg` | 14px | field labels, list items |
| `--text-xl` | 16px | section titles |
| `--text-2xl` | 18px | dialog titles |

`--leading-tight` (1.3) is for headings, buttons and hints;
`--leading-normal` (1.5) for body text. `--font-ui` is the system's UI
font, for the window's own text; `--font-mono` is for code, output, paths,
addresses and tokens. It is JetBrains Mono, which ships with the app
(`@fontsource/jetbrains-mono`), so it looks the same on every machine.
A font the app bundles needs its licence in `public/licenses/`. [Foundations / Type](?path=/story/foundations-type--scale).

### Space, radii and shadows

Spacing runs on a 4px grid: `--space-1` (4px) to `--space-8` (32px), and
`--space-half` (2px) for hairline gaps. Common layouts:

- between a label and its hint: `--space-half`;
- between buttons, and an icon and its text: `--space-2`;
- a field row's padding, in a list: `--space-2` top and bottom;
- a dialog's padding: `--space-5` at the sides.

Corners: `--radius-sm` (4px) for badges and small controls, `--radius-md`
(6px) for buttons, fields and boxes, `--radius-lg` (12px) for dialogs.
Shadows: `--shadow-popover` for menus and completion, `--shadow-dialog`
for dialogs. [Foundations / Space, radii and shadows](?path=/story/foundations-space-radii-and-shadows--spacing).

## Parts

| Part | For | Story |
| --- | --- | --- |
| `Button` | Every button. `variant`: `primary` for the one main action in a view, `secondary` for others, `quiet` for minor ones such as Cancel. `size`: `md`, or `sm` beside fields. An icon-only button takes a `label`, already translated. | [UI / Button](?path=/story/ui-button--variants) |
| `SegmentedControl` | A choice of one from two to five short options, shown at once. Use `Select` for more, or for long labels. | [UI / SegmentedControl](?path=/story/ui-segmentedcontrol--options) |
| `FieldRow` | A field's label, a one-line hint, and its control. `divided` in lists such as Settings; `wide` puts a long control under the text. | [UI / Fields](?path=/story/ui-fields--rows) |
| `TextField` | Text input that saves on blur or Enter, with Esc to undo. `mono` for paths, addresses and tokens. | [UI / Fields](?path=/story/ui-fields--rows) |
| `Select` | A choice from a list. | [UI / Fields](?path=/story/ui-fields--rows) |
| `Checkbox` | On or off. | [UI / Fields](?path=/story/ui-fields--rows) |
| `StatusBox` | The state of something, with a tone (`ok`, `warn`, `err`, `busy`, `info`) and an optional second line. | [UI / StatusBox](?path=/story/ui-statusbox--all-tones) |
| `Dialog` | A dialog with a title, a body and a footer of buttons. `DialogOverlay` and `DialogFrame` build one with its own layout, as Settings does. Esc and a click outside close it, and focus goes back to what opened it. | [UI / Dialog](?path=/story/ui-dialog--small) |
| `PanelHeader` | The header of a panel or bar (variables, docs, log, find): a title or controls on the left, actions and the one close button on the right. `closeLabel`, already translated, names the close button. | [UI / PanelHeader](?path=/story/ui-panelheader--title-and-actions) |
| `ContextMenu` | A menu at the pointer, such as the cell commands on a right-click. Up and Down move between enabled items; Escape, a click outside or a scroll close it. Items carry their shortcut in the platform's names. | [UI / ContextMenu](?path=/story/ui-contextmenu--groups) |

The parts hold no text of their own: callers pass translated text, so
every string goes through Lingui with a translator note at the call. Their
styles are in `src/styles/ui.css`, with `ui-` class names.

To add a part: write it in `src/components/ui/` with its rules in
`ui.css` (tokens only), give it a story in `src/stories/ui/` tagged
`themes`, list it in the table above, and test what it does.

## Native parts

Each theme sets `color-scheme` in `tokens.css`, so the parts the webview
draws itself, such as scrollbars and their tracks, are dark in the dark
themes.

## The window's frame on Windows

On Windows the page draws the title bar and the menu bar, as Windows 11's
own apps do (`src/lib/window-frame.ts`). The system's menu bar is the
Win32 kind, in the system's menu font, colour and spacing, and its title
bar sat above a separate row of tabs. The window is made without the
system's frame (`notebook_window` in `src-tauri/src/commands/window.rs`):

- `TitleBar`: the app's icon, the tabs, and `WindowControls` (minimise,
  maximise or restore, close, in Segoe Fluent Icons at Windows 11's 46px).
  The space between the tabs moves the window; a double click maximises it.
- `MenuBar`: built from the same model as the native menu bar elsewhere
  (`menuModel`), on the left of the row the toolbar is in. It never takes
  the keyboard's focus, as a native menu doesn't, so Undo, Cut and Copy act
  where the keyboard was. Alt highlights File; Alt and a menu's first
  letter opens it; the arrows, Enter and Escape work in it.

Two shades, as on Windows 11: the title bar is `--bg-secondary`, and the
active tab, the menu and tool row, the notebook, and the bars and panels
around it (find, variables, docs, log, status, shortcut hints) are
`--bg-primary`, with borders between them. The
UI font there is Segoe UI Variable (`[data-platform="windows"]` in
`tokens.css`). The `--title-bar-height`, `--caption-*` and
`--radius-flyout` tokens are Windows 11's sizes and its close button's
red.

## The window's frame on macOS

On macOS the system draws the menu bar, at the top of the screen, and the
traffic lights. The page draws the rest of the title bar, with the tabs in
it beside the traffic lights, as Safari and Xcode do; before, the system's
title bar showed "Aximar" in a row of its own above the tabs. The window's
content runs up to its top edge (`TitleBarStyle::Overlay`, with the title
hidden, in `notebook_window`), and the traffic lights sit in the middle of
the page's 38px bar.

- `TitleBar` leaves `--traffic-light-space` at its left for the traffic
  lights, or none in full screen, where the system hides them. The space
  between the tabs moves the window; a double click zooms it.
- The tabs are rounded buttons, as Safari's compact tabs are: the active
  one is `--bg-primary` with a hairline shadow, and a tab's close button
  shows while the pointer is over it. While another window has the focus
  the active tab goes flat and the titles grey, as the traffic lights do.

One shade for the title bar and the toolbar beneath it (`--bg-secondary`),
as a unified title bar and toolbar are, with a hairline under the toolbar.
The `--mac-*` tokens hold the bar's and the tabs' sizes.

## Icon buttons

An icon alone is a `Button` with `icon` and `label`: the label names it
for screen readers and is its tooltip. A button for a command passes
`title` too, the label and the command's shortcut from
`formatShortcut` (`src/commands/shortcut.ts`), so the tooltip shows the
platform's names (⌘ on macOS, Ctrl elsewhere). The toolbar's buttons are
built this way from the command list (`src/commands/registry.ts`); a
toggle for a panel sets `aria-pressed` and is tinted while the panel is
open, never filled.

## Icons

From [Tabler](https://tabler.io/icons), imported only through
`src/components/ui/icons.ts`, where each icon has a note on where it's
used. To use a new icon, add it there with its note. Sizes: 14px inline in
text, 16px in buttons and lists, 20px in the toolbar, all with a stroke of
1.75. [Foundations / Icons](?path=/story/foundations-icons--in-use).

## Checks

- **The token check**, `npm run check:tokens`, fails on a raw colour, font
  size in px or corner radius in px in a stylesheet, beyond the baseline in
  `scripts/css-tokens-baseline.json`, and on a Tabler import outside
  `icons.ts`. Its message names the token with the same value. When you
  move rules onto tokens, the counts fall and the check asks you to run
  `npm run check:tokens -- --update`, which records them; the baseline
  never goes up.
- **The contrast test**, `src/styles/tokens.test.ts`, holds each pair in
  `contrast-pairs.ts` to 4.5:1 in every theme, and print text to 4.5:1 on
  white.
- **The gallery**, `npm run i18n:gallery`, shoots every story, the
  Foundations and UI stories in the light and dark themes, and fails if a
  `SegmentedControl` label wraps or overflows in any language.

Moving an old screen onto the parts: give it a story first if it has
none, take the gallery, move it, compare the screenshots, and lower the
baseline in the same change.
