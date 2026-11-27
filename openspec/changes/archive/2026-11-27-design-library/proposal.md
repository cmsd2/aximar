## Why

New UI in Aximar starts from a blank rule in a 3,540-line stylesheet. That file holds 9 font sizes, 64 distinct paddings, 9 corner radii, 25 button classes and 39 raw colours (24 distinct) outside the theme blocks, so each new dialog picks its own values and drifts from its neighbours. `docs/ui-audit.md` ranks this ("The system underneath", items 5 to 7) and asks for tokens next (its suggested order, step 3). The Settings window just gained the first shared parts (`StatusBox`, a segmented control, a setting row), and the user wants them, and the tokens under them, in Storybook "so future UI elements look the same".

## What Changes

- The theme tokens move from `src/styles/global.css` to `src/styles/tokens.css`, with their current values, and gain scales: a type scale of 6 steps with 2 line heights, spacing on a 4px grid, 3 corner radii, 2 shadows, and `--font-ui` beside `--font-mono`.
- Storybook gains a **Foundations** section: the colour tokens as swatches in light and dark with each text pairing's contrast ratio, the type scale, the spacing scale, the radii and shadows, and the icon set.
- Storybook gains a theme switch (light, dark) beside the language switch, and the gallery screenshots the Foundations and component stories in both themes.
- Shared primitives move to `src/components/ui/`, each with a story showing its states: `Button` (primary, secondary, quiet, icon-only, disabled), `SegmentedControl` (today's `ChoiceButtons`), `StatusBox`, `FieldRow` (today's `SettingRow`), `TextField` (today's `TextSetting`), `Select`, `Checkbox` and `Dialog` (frame, header, body, footer). The Settings window and the Export LaTeX dialog use them.
- Tabler icons (`@tabler/icons-react`, which Settings already uses) become the app's icon set, imported through one file that records each icon's use, answering the audit's step 5.
- A **Guide** story says when to use each primitive and states the rule: new UI uses tokens and primitives, never a raw colour or a one-off size.
- A check, `scripts/check-css-tokens.mjs`, fails CI when a rule adds a raw colour, font size or radius outside `tokens.css`. It holds today's count as a baseline that may only go down.
- A test fails when a text and background pairing in either theme falls below 4.5:1.
- JetBrains Mono ships with the app (under the OFL), so `--font-mono` looks the same on every machine.

## Capabilities

### New Capabilities

- `design-library`: the tokens, the Foundations and component stories, the primitives' contract, the icon set, and the checks that keep new UI on them.

### Modified Capabilities

None. No existing spec covers styling or Storybook.

## Impact

- **Frontend**: `src/styles/tokens.css` (new), `src/styles/global.css` (token blocks move out; the Settings and Export LaTeX rules move onto the primitives), `src/components/ui/` (primitives), `src/components/settings/`, `src/components/ExportLatexModal.tsx`, `src/assets/fonts/`.
- **Storybook**: `.storybook/preview.tsx` (theme switch), `src/stories/foundations/`, `src/stories/ui/`, `scripts/i18n-gallery.mjs` (a theme per screenshot).
- **Tools and CI**: `scripts/check-css-tokens.mjs` and its baseline, a vitest contrast test, and a step in the CI gallery job.
- **Docs**: `docs/design-library.md`, and `docs/ui-audit.md` marks items 5 and 7 as addressed.
- **Core, MCP, Tauri**: none.
- **Issues**: none filed yet; this answers `docs/ui-audit.md` items 5, 7 and 14 and its suggested order, steps 3 and 5.

## Non-goals

- Moving every existing rule onto the tokens. The check's baseline lets that happen screen by screen, in later changes (the audit's step 3, second half, and step 4 for the notebook).
- Choosing one palette for both themes (audit item 6). The tokens keep today's values; the Foundations page makes a later palette change easy to see and review.
- A component library such as Radix UI. The primitives here are thin wrappers over native elements; a library can come when the app needs popovers or menus it can't build well itself.
- Publishing the tokens or primitives as an npm package for the VS Code extension's renderers. `tokens.css` stands alone so a later change can move it to `packages/`.
- Storybook's MDX docs pages. Foundations and the Guide are ordinary stories, so the gallery screenshots them and the language switch applies.
