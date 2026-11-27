## 1. Tokens and Foundations (first PR)

- [x] 1.1 Move the token blocks and the print palette from `src/styles/global.css` to `src/styles/tokens.css` unchanged, and import it at the top of `global.css`; verify the gallery's screenshots match the ones before the move, pixel for pixel
- [x] 1.2 Add the type, line-height, spacing, radius and shadow scales and `--font-ui` to `tokens.css`, used by nothing yet; verify `npm run build` and the stories still render
- [x] 1.3 Add `src/styles/contrast-pairs.ts` and `src/styles/tokens.test.ts`: a block parser for `tokens.css`, the WCAG ratio, and the 4.5:1 assertion per pair and theme; verify with the tests `every_colour_has_a_dark_value`, `the_two_dark_blocks_agree`, `text_pairs_reach_4_5_to_1`, and a fixture test `a_low_contrast_pair_fails` using `#2d8a4e` on `#edf7f0`
- [x] 1.4 Add the `theme` global to `.storybook/preview.tsx`, setting `data-theme` on the root; verify by hand that Dark turns a story dark, and with a vitest test of the decorator that checks `data-theme` follows the global
- [x] 1.5 Add the Foundations stories (`src/stories/foundations/`): colours with ratios, type, spacing, radii and shadows, reading `tokens.css` through `?raw`; tag them `themes` and `english-only`; verify each renders in both themes and that a token added to `tokens.css` shows without a story edit
- [x] 1.6 Teach `scripts/i18n-gallery.mjs` the `themes` and `english-only` tags; verify the gallery has light and dark screenshots of each Foundations story and none in Spanish or the pseudo-locale

## 2. The token check (one PR)

- [ ] 2.1 Write `scripts/check-css-tokens.mjs` with `--update`, and `scripts/css-tokens-baseline.json` with today's counts; verify with `node --test` tests over fixture stylesheets: `counts_hex_rgb_and_hsl`, `ignores_comments_and_url_fragments`, `ignores_percent_and_zero_radii`, `fails_above_the_baseline`, `passes_and_reports_below_it`, `update_never_raises_a_count`, `names_the_matching_token`
- [ ] 2.2 Add the check for direct `@tabler/icons-react` imports, and `src/components/ui/icons.ts` with the icons Settings uses, each with a `use` note; move Settings' imports onto it; verify the test `flags_a_direct_icon_import` and that the check passes on the tree
- [ ] 2.3 Add `npm run check:tokens` and a step in the CI `test` job; verify the job fails on a branch that adds `color: #333;` and passes on this one

## 3. Primitives (one PR)

- [ ] 3.1 Add `src/styles/ui.css` and `Button`, `Select`, `Checkbox` with stories of every state, tagged `themes`; verify vitest tests `button_variants_render_their_classes`, `icon_only_button_needs_a_label` (a type test with `@ts-expect-error`), and that `ui.css` has a baseline of zero in the check
- [ ] 3.2 Move `StatusBox`, and `ChoiceButtons`, `SettingRow` and `TextSetting` as `SegmentedControl`, `FieldRow` and `TextField`, into `src/components/ui/` with stories; add arrow-key movement to `SegmentedControl`; verify the existing Settings tests pass unchanged apart from imports, and new tests `segmented_arrow_keys_move_the_choice` and `segmented_labels_do_not_wrap_in_pseudo`
- [ ] 3.3 Add `DialogOverlay`, `DialogFrame` and `Dialog` with stories; verify tests `esc_closes`, `a_click_outside_closes`, `focus_returns_to_the_opener`
- [ ] 3.4 Move the Settings window and Export LaTeX onto the primitives and delete the rules they no longer use; lower the baseline; verify the gallery shows both screens with the same layout as before, by comparing the screenshots, and that `npm run check:tokens` reports lower counts

## 4. The monospace font (one PR)

- [ ] 4.1 Add `@fontsource/jetbrains-mono` 5.3.0, pinned, import its regular and bold weights in `src/main.tsx` and `.storybook/preview.tsx`, and point `--font-mono` at it; verify the gallery script's `document.fonts.check` assertion on a code cell story, and that `npm run tauri build` bundles the woff2 files (they appear in `dist/assets`)

## 5. The guide (with group 3's PR)

- [ ] 5.1 Write `docs/design-library.md` and the Guide story that renders it with `react-markdown`; verify every primitive in `src/components/ui/` is named with a working link to its story, by a vitest test that compares the doc's links with the directory listing
- [ ] 5.2 Mark items 5, 7 and 14 of `docs/ui-audit.md` as addressed by this change, and add a line on the design library to `CLAUDE.md`'s project rules ("New UI uses the tokens and primitives; see docs/design-library.md"); verify `openspec validate design-library --strict` passes
