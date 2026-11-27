## Purpose

Aximar's design library: the tokens every rule draws its colours, sizes and spacing from, the shared UI primitives built on them, their stories in Storybook, and the checks that keep new UI on them, so new screens look like the old ones.

## ADDED Requirements

### Requirement: Tokens are defined once

`src/styles/tokens.css` SHALL define every design token as a CSS variable, and no other stylesheet SHALL define one. Layout variables the app sets while it runs, such as `--font-size-mono`, `--markdown-indent` and `--gutter-width`, are not design tokens and stay where they are. The tokens SHALL be:

- colours, each with a light value and a dark value, applied for the `light` and `dark` themes and for `auto` by the system's setting;
- a print palette, applied in `@media print`;
- a type scale of 6 sizes (`--text-xs` to `--text-2xl`) and 2 line heights (`--leading-tight`, `--leading-normal`);
- a spacing scale on a 4px grid (`--space-1` = 4px up to `--space-8` = 32px);
- 3 corner radii (`--radius-sm`, `--radius-md`, `--radius-lg`);
- 2 shadows (`--shadow-popover`, `--shadow-dialog`);
- 2 font stacks (`--font-ui`, `--font-mono`).

#### Scenario: A theme sets every colour

- **WHEN** the tokens file is parsed
- **THEN** each colour token has a value in the light block and in the dark block, and the dark block appears both under `prefers-color-scheme: dark` and under `[data-theme="dark"]` with the same values

#### Scenario: No other file defines a token

- **WHEN** every stylesheet under `src/` is searched for a definition of a colour, type, spacing, radius, shadow or font variable
- **THEN** only `src/styles/tokens.css` defines one

### Requirement: Text meets 4.5:1 contrast in both themes

Each text colour token SHALL be listed with the background tokens it appears on, and each pairing SHALL reach a contrast ratio of at least 4.5:1 in the light theme and in the dark theme. Placeholder text and decoration such as icon tints are exempt and SHALL be marked so in the list.

#### Scenario: A low-contrast pairing fails the test

- **WHEN** the success text colour is set to `#2d8a4e` on the light `--success-bg` of `#edf7f0` (3.95:1)
- **THEN** the contrast test fails, naming the pairing, the theme and the ratio

### Requirement: New rules use tokens, not raw values

A check SHALL find, outside `tokens.css`, the declarations that write a colour as hex, `rgb()` or `hsl()`, a font size in px, or a corner radius in px, in each stylesheet. A baseline in the repository SHALL record each such declaration and how often it appears. The check SHALL fail on a declaration the baseline doesn't hold, or holds fewer times. When one appears less often than its entry says, the check SHALL fail until the baseline records the lower count, and its update SHALL never add or raise an entry. CI SHALL run the check.

#### Scenario: A new raw colour fails

- **WHEN** a change adds `color: #333;` to a rule in `global.css`
- **THEN** the check fails, naming the file, the line and the token to use instead if one has that value

#### Scenario: Moving a rule onto tokens lowers the baseline

- **WHEN** a change replaces three raw font sizes with `--text-sm`
- **THEN** the check reports the lower count and fails until the change records it with the check's update option, which then passes

### Requirement: Storybook shows the foundations

Storybook SHALL have a Foundations section with a story each for colours, type, spacing, radii and shadows, and icons. The colours story SHALL show each token's swatch, name, and light and dark values, and each text pairing's ratio. The type story SHALL show each step's name, size and line height in the UI font and the monospace font.

#### Scenario: A new token appears without editing the story

- **WHEN** a colour token is added to `tokens.css`
- **THEN** the colours story shows it, because the story reads the token list rather than repeating it

### Requirement: Storybook switches theme

Storybook SHALL offer a theme switch with Light and Dark beside the language switch, and every story SHALL render in the chosen theme. The gallery SHALL screenshot the Foundations and component stories in both themes, and the other stories in the light theme.

#### Scenario: Dark from the toolbar

- **WHEN** the reader chooses Dark in the theme switch
- **THEN** the story's root carries `data-theme="dark"` and the page background is the dark `--bg-primary`

#### Scenario: Both themes in the gallery

- **WHEN** `npm run i18n:gallery` runs
- **THEN** the gallery holds a light and a dark screenshot of each Foundations and component story

### Requirement: Primitives are shared and documented

`src/components/ui/` SHALL hold the shared primitives: `Button`, `SegmentedControl`, `StatusBox`, `FieldRow`, `TextField`, `Select`, `Checkbox` and `Dialog`. Each SHALL take its colours, sizes and spacing from tokens only, and SHALL have a story showing each of its states, which the gallery screenshots in English and the pseudo-locale. A primitive SHALL NOT contain user-facing text of its own except through Lingui with a translator note.

#### Scenario: Button states

- **WHEN** the Button story renders
- **THEN** it shows primary, secondary, quiet, icon-only and disabled buttons, each also with keyboard focus

#### Scenario: A long translation in a primitive

- **WHEN** the SegmentedControl story renders in the pseudo-locale
- **THEN** no label wraps onto a second line or overflows its button

### Requirement: The app uses the primitives where they exist

The Settings window and the Export LaTeX dialog SHALL use the primitives instead of their own button, field, status and frame rules, and their appearance SHALL not change beyond the primitives' own style.

#### Scenario: Export LaTeX keeps its look

- **WHEN** the Export LaTeX story is screenshotted before and after the change
- **THEN** the dialog has the same size and layout, with its buttons drawn by `Button`

### Requirement: One icon set

Icons in the window SHALL come from Tabler (`@tabler/icons-react`), at the sizes the icon story lists, and SHALL be imported through `src/components/ui/icons.ts` only, which records each icon's use. The token check SHALL fail on an import of `@tabler/icons-react` anywhere else. New UI SHALL NOT use text characters such as ×, ⓘ, ⚠, ▸ or ▾ as icons.

#### Scenario: The icon story lists the icons in use

- **WHEN** the icons story renders
- **THEN** it shows every icon `icons.ts` exports, with its name and its recorded use

#### Scenario: A direct import fails

- **WHEN** a component imports `IconTrash` from `@tabler/icons-react` itself
- **THEN** the token check fails and names `src/components/ui/icons.ts`

### Requirement: The monospace font ships with the app

The app SHALL bundle JetBrains Mono and use it for `--font-mono`, so code and output render in the same font on macOS, Windows and Linux.

#### Scenario: No system monospace fallback in the window

- **WHEN** a code cell renders in a story
- **THEN** the browser reports JetBrains Mono as the font of its text

### Requirement: A guide says how to use the library

Storybook SHALL have a Guide story, and `docs/design-library.md` SHALL hold the same rules: which primitive to use for which job, which token to use for text, surfaces, borders and status, the type and spacing steps for common layouts, and the rule that new UI uses tokens and primitives.

#### Scenario: The guide links each primitive

- **WHEN** the Guide story renders
- **THEN** it names every primitive in `src/components/ui/` with a link to its story
