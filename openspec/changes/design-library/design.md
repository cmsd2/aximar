## Context

All of the window's styling lives in `src/styles/global.css` (3,540 lines), which `src/App.tsx` and `.storybook/preview.tsx` import. Its theme tokens sit in three blocks at the top (`:root`, the dark block under `prefers-color-scheme`, and `[data-theme="dark"]`), plus a print block that overrides some of them with `!important`. Below those, rules mix tokens with raw values:

| Kind | Distinct values | Most used |
| --- | --- | --- |
| Font size | 9 (10 to 20px) | 12px ×40, 13px ×36, 11px ×24 |
| Corner radius | 9 | 4px ×19, 6px ×19, 3px ×8, 12px ×7 |
| Padding | 64 | |
| Gap | 8 | 8px ×17, 6px ×9, 4px ×7 |
| Raw colour outside the token blocks | 24 (39 uses) | `#333`, `#999`, `#fff` |

There are 25 button classes, from `.toolbar-btn` to `.variable-kill-all-btn`. Dialogs share `.palette-overlay`, and Export LaTeX borrows `.settings-modal`.

Storybook 10 (`.storybook/`) renders the window's components with Tauri mocked, a language switch (English, Spanish, the pseudo-locale), and a gallery script (`scripts/i18n-gallery.mjs`) that screenshots every story in each language. It has no theme switch and no docs pages.

The settings-window change added the first shared parts: `src/components/ui/StatusBox.tsx`, and `SettingRow`, `ChoiceButtons` and `TextSetting` in `src/components/settings/SettingRow.tsx`. It also added `@tabler/icons-react` 3.51.0 and the tokens `--success-bg`, `--success-text`, `--warn-bg` and `--selected-bg`.

ADR 009 (shared packages) puts design tokens in CSS custom properties, in a package apps import from source. Aximar shares TypeScript with the VS Code extension through npm packages under `packages/`, and its output renderers are meant to look the same in every front end.

## Goals / Non-Goals

**Goals:**

- One file of tokens, with scales, that new rules draw from.
- Stories that show the tokens and every primitive, in both themes, so a reviewer sees a change in the gallery.
- Primitives the next screen can use, so its author doesn't write a button rule.
- A check that stops the raw-value count rising, without first rewriting the stylesheet.

**Non-Goals:**

- Moving every existing rule onto tokens, or choosing one palette (see the proposal).
- An npm package for the tokens or primitives.
- A third-party component library.

## Decisions

### 1. `tokens.css` beside `global.css`, imported by it

`src/styles/tokens.css` holds the token blocks, moved from `global.css` unchanged, the print palette, and the new scales. `global.css` starts with `@import "./tokens.css";`, so every place that imports `global.css` keeps working, and Vite inlines the import. The file uses only `:root`, `[data-theme]`, `@media (prefers-color-scheme)`, `@media print` and `@font-face`, so a later change can move it to `packages/` for the VS Code renderers.

Layout variables the app sets while it runs (`--font-size-mono`, `--markdown-indent`, `--gutter-width`, `--print-font-size`) stay where they are: they hold settings, not design decisions.

### 2. The scales come from today's values

The scales keep the sizes the stylesheet uses most, so moving a rule onto a token rarely changes how it looks:

| Token | Value | Replaces |
| --- | --- | --- |
| `--text-xs` | 11px | 10px, 11px |
| `--text-sm` | 12px | 12px |
| `--text-md` | 13px | 13px (body) |
| `--text-lg` | 14px | 14px, 15px |
| `--text-xl` | 16px | 16px (section titles) |
| `--text-2xl` | 18px | 18px, 20px (dialog titles) |
| `--leading-tight` | 1.3 | headings, buttons |
| `--leading-normal` | 1.5 | body text |
| `--space-half` | 2px | hairline gaps |
| `--space-1` to `--space-8` | 4px to 32px | gaps and paddings on the 4px grid; 6px and 10px round to 4px or 8px and 8px or 12px |
| `--radius-sm` | 4px | 3px, 4px |
| `--radius-md` | 6px | 6px, 8px |
| `--radius-lg` | 12px | 10px, 12px |
| `--shadow-popover` | `0 4px 16px rgba(0,0,0,.18)` | completion and menus |
| `--shadow-dialog` | `0 16px 48px rgba(0,0,0,.2)` | dialogs |

`50%` and `0` stay as written: they describe a shape, not a size, and the check ignores them.

### 3. A ratchet, not a rewrite

`scripts/check-css-tokens.mjs` (Node, no dependencies) reads each `.css` file under `src/` except `tokens.css`, blanks out comments, and finds the declarations that write a raw value:

- a colour: `#` followed by 3 to 8 hex digits outside `url()`, or `rgb(`, `rgba(`, `hsl(`, `hsla(`;
- a `font-size` in px;
- a `border-radius` (or one corner's) with a px value other than 0.

`scripts/css-tokens-baseline.json` records each such declaration, as `property: value`, and how often it appears in each file. Keying by declaration rather than by a count per file lets the check name the new one exactly: a declaration the baseline doesn't hold fails at its line, and one it holds fewer times fails with one line saying how many times it appears and how many the baseline allows. Either way the message names the token whose light value matches, such as `use --text-sm`.

A declaration that appears less often than its entry says also fails, until `--update` records the lower count, so the baseline never sits above the tree. `--update` never adds or raises an entry. `--init` records a tree with no baseline. It also lists every `.ts` and `.tsx` file that imports `@tabler/icons-react`, other than `src/components/ui/icons.ts`.

`npm run check:tokens` runs it, and CI runs it, with its tests (`node --test scripts/check-css-tokens.node-test.mjs`), in the `test` job after `npm ci`. The first baseline holds 254 raw values, all in `global.css`: 134 font sizes, 61 radii and the rest colours.

Alternative: Stylelint with `declaration-strict-value`. It catches the same values, but it adds a dependency tree and a config to keep, and it can't hold a per-file baseline without a plugin. Rejected for now. The script is about 100 lines, and Stylelint can replace it if the rules grow.

### 4. Contrast as a unit test

`src/styles/contrast-pairs.ts` lists each text token with the background tokens it sits on, marking placeholder and decoration pairs as exempt. `src/styles/tokens.test.ts` parses `tokens.css` with a small block parser, resolves each pair's values in the light and the dark block, and asserts at least 4.5:1, using the WCAG relative-luminance formula. The colours story imports the same list, so the ratios it shows are the ones the test checks.

The first run found three pairings the audit's stories hadn't shown: green status text (`--success-color`, 4.32:1 on white), the running colour used as text (`--running-color`, 3.25:1), and accent text on `bg-secondary` and `bg-cell` (4.31:1 and 4.50:1). The light `--success-color` becomes `#237a42` and the light `--running-color` `#8a6508`, the green and amber the syntax colours already use. Accent text stays listed only on `bg-primary` and `bg-input`, where it reaches 4.7:1. The print palette had kept the old `--text-secondary` and `--error-color` (`#e3342f`, 4.4:1 on paper) and takes the light values; a test checks print text on white.

### 5. The theme switch is a Storybook global

`.storybook/preview.tsx` gains a `theme` global (Light, Dark) beside `locale`. The existing decorator sets `document.documentElement.dataset.theme` from it, which the token blocks already respond to. Stories that should appear in both themes in the gallery carry the tag `themes`. `scripts/i18n-gallery.mjs` reads each entry's tags from `index.json`, and screenshots tagged stories with `theme:light` and `theme:dark` in English and the pseudo-locale. Other stories are screenshotted as today, in light. Foundations stories carry the tag `english-only` as well, since their text is token names and values.

### 6. Foundations are stories, not MDX

The Foundations stories (`src/stories/foundations/`) are ordinary TSX stories, so the gallery screenshots them, the theme switch applies, and no docs addon is needed. They read the tokens at run time from `getComputedStyle(document.documentElement)` and from a token list parsed out of `tokens.css` (imported with Vite's `?raw`), so a new token appears without a story edit. The icons story renders `src/components/ui/icons.ts`, whose exports carry a `use` note each.

### 7. The primitives

All under `src/components/ui/`, styled in `src/styles/ui.css` with `ui-` class names, so they never collide with the older rules. `ui.css` uses tokens only, and its baseline in the check is zero.

| Primitive | From | Shape |
| --- | --- | --- |
| `Button` | `.dialog-primary-btn`, `.settings-token-btn`, `.template-skip` | `variant`: primary, secondary, quiet; `size`: sm, md; optional `icon`; icon-only buttons require `label`, used for `aria-label` and `title` |
| `SegmentedControl` | `ChoiceButtons` | a radio group; arrow keys move the choice; labels never wrap, and the group scrolls sideways before it squeezes |
| `StatusBox` | as is | `tone`: ok, warn, err, busy, info |
| `FieldRow` | `SettingRow` | label, hint and control, with ids wired for `htmlFor` and `aria-describedby`; `wide` stacks the control under the text |
| `TextField` | `TextSetting` | saves on blur or Enter, Esc restores; `mono` for paths and addresses |
| `Select` | the styled `<select>`s | native `<select>` with the app's border and focus ring |
| `Checkbox` | `.settings-checkbox` | native checkbox with the accent colour |
| `Dialog` | `.settings-modal`, `.palette-overlay` | `DialogOverlay` and `DialogFrame` (`size` sm, md, lg) for custom layouts such as Settings, and `Dialog` (title, body, footer) for the rest; Esc and a click outside close it, and it returns focus to what opened it |

The primitives hold no text of their own except the icon-only button's label, which the caller passes already translated. `Dialog`'s close button takes its label the same way.

The Settings window and Export LaTeX move onto them in this change, as the first users, with before and after gallery screenshots to compare. The rest of the window moves in later changes, screen by screen, each lowering the baseline.

Moving those two screens took the baseline from 254 raw values to 232. The comparison shows the same layout, with three differences from rounding onto the scales: setting rows are 2px tighter (10px padding to `--space-2`), Export LaTeX's labels grow from 13px to 14px (`--text-lg`, as in Settings), and a disabled button now looks disabled, which the old Settings buttons didn't. A segmented group too narrow for its labels scrolls sideways rather than wrapping them, and the gallery fails if a label wraps in any language. The Settings window keeps its own grid inside `DialogFrame`, through the `settings-window` class.

### 8. Tabler through one file

`src/components/ui/icons.ts` re-exports each icon the app uses, under the name it uses, with a `use` note:

```ts
/** Settings sidebar: General */
export { IconAdjustments } from "@tabler/icons-react";
```

The icons story lists these by parsing the module's exports and their doc comments from the `?raw` source. Icons render at 14px inline in text, 16px in buttons and lists, and 20px in the toolbar, with a stroke of 1.75. Importing one icon at a time keeps the bundle to the icons used.

### 9. JetBrains Mono from Fontsource

`@fontsource/jetbrains-mono` (5.3.0, pinned) supplies the regular and bold weights as woff2, with the OFL. `tokens.css` declares `--font-mono: "JetBrains Mono", ui-monospace, Menlo, Consolas, monospace;`. The import goes in `src/main.tsx` and `.storybook/preview.tsx`, so both bundle the files and nothing loads over the network. The gallery script asserts `document.fonts.check("13px 'JetBrains Mono'")` on a code cell story.

The OFL lets the font ship with the app if its copyright notice and licence travel with it. `public/licenses/` holds `JetBrainsMono-OFL.txt`, copied from the package, and Vite copies it into the built app. Computer Modern, which the app already bundled for the Markdown font under the same licence, had no notice in the bundle; `ComputerModern-OFL.txt` goes beside it, with a README naming both. The editor theme and the Markdown "Mono" font, which wrote out their own copies of the old monospace stack, now use `var(--font-mono)`. The bundle grows by 224 KB: both weights in five subsets, as woff2 with woff fallbacks.

`--font-ui` stays the system stack (`-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`), as each platform's own UI font is what users expect for chrome.

### 10. The guide is the doc

`docs/design-library.md` holds the guide: which primitive for which job, which token for text, surfaces, borders and status, common layouts in tokens, and the rule. The Guide story imports it with `?raw` and renders it with `react-markdown`, which the window already uses, so there is one copy. Its links to stories use Storybook paths (`?path=/story/ui-button--states`), which work from the story and read as plain paths in the file.

### What changes for MCP clients and saved notebooks

Nothing. Styles reach neither.

## Risks / Trade-offs

- **[Rounding the scales shifts some screens by a pixel or two]** → Only rules moved onto tokens change, a screen at a time, and the gallery shows each before and after.
- **[The baseline hides old raw values]** → It only goes down, and `docs/ui-audit.md` lists where they are. The check prints the token that matches a raw value, so moving a rule is mostly mechanical.
- **[Parsing CSS with regular expressions]** → The stylesheet has no id selectors and no colours in selectors or strings, so `#` plus hex digits only matches colours. The check has unit tests over a fixture stylesheet for the cases it must ignore (comments, `url(#...)`, `50%`).
- **[A bundled font adds 224 KB]** → Two weights, which the installer already dwarfs.
- **[Dark screenshots double the gallery's time for tagged stories]** → Only Foundations and UI stories get both themes, about 15 stories.
