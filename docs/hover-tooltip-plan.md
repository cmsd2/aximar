# Function Hover Tooltip

> **Status**: Fully implemented.

## Context

Users want to hover over function names in code cells to see what they do, with the option to jump to full documentation. The cell input is a plain `<textarea>` so we can't attach hover listeners to individual words — instead we detect the word under the mouse cursor on `mousemove`, look it up in the catalog via the existing `getFunction(name)` backend command, and show a positioned tooltip.

All the infrastructure exists: `getWordAtCursor()` and `getCaretCoordinates()` in `src/lib/textarea-caret.ts`, the `getFunction()` client in `src/lib/catalog-client.ts`, and the `MaximaFunction` type with signatures, descriptions, examples, and see_also.

## Approach

- **New utility** `getWordAtPosition()` in `textarea-caret.ts`: converts mouse (x, y) over a textarea into the word at that position using a mirror-div technique.
- **New hook** `useHoverTooltip(textareaRef)`: on debounced `mousemove`, gets the word under the mouse, calls `getFunction()` (with a local cache), and exposes tooltip state.
- **New component** `HoverTooltip`: fixed-position card showing signature, description, category, and a "Docs" link.
- **"Docs" link** opens the command palette pre-filled with the function name via a new optional `initialQuery` prop.

## Changes

### 1. `src/lib/textarea-caret.ts` — Add `getWordAtPosition(textarea, mouseX, mouseY)`

Convert mouse coordinates to a character offset using a mirror div positioned over the textarea, then extract the word at that position. Returns `{ word: string, start: number } | null`.

### 2. `src/hooks/useHoverTooltip.ts` — New hook

State: `{ func: MaximaFunction | null, x: number, y: number, visible: boolean }`

- `onMouseMove(e)`: debounced 150ms. Calls `getWordAtPosition()`. If word changed, looks up in cache or calls `getFunction(word)`. Shows tooltip at mouse position if found.
- `onMouseLeave()`: hides tooltip.
- Hides when autocomplete is active (passed as param).
- Cache: `Map<string, MaximaFunction | null>` to avoid repeated backend calls.

### 3. `src/components/HoverTooltip.tsx` — New component

Props: `{ func: MaximaFunction, x: number, y: number, onViewDocs: (name: string) => void }`

Layout:
- First signature in monospace
- Description (truncated to 2 lines)
- Category as small label
- "Docs →" link button

### 4. `src/styles/global.css` — Tooltip styles

- `.hover-tooltip`: `position: fixed`, `z-index: 60`, `max-width: 320px`, shadow, border-radius, theme-aware bg/text
- Sub-elements for signature, description, category, link

### 5. `src/components/Cell.tsx` — Wire up

- Call `useHoverTooltip(textareaRef)`
- Add `onMouseMove` and `onMouseLeave` handlers to textarea
- Render `<HoverTooltip>` when visible
- Pass a callback that opens command palette with the function name

### 6. `src/components/CommandPalette.tsx` — Accept `initialQuery` prop

- Add optional `initialQuery?: string` prop, use it as the starting `query` state.

### 7. `src/App.tsx` — Pass palette query state

- Add `paletteQuery` state alongside `paletteOpen`
- Cell's "view docs" callback sets both `paletteQuery` and opens the palette
- Pass `initialQuery` to `CommandPalette`

## Files

| File | Action |
|---|---|
| `src/lib/textarea-caret.ts` | Add `getWordAtPosition()` |
| `src/hooks/useHoverTooltip.ts` | New — hover detection hook |
| `src/components/HoverTooltip.tsx` | New — tooltip component |
| `src/components/Cell.tsx` | Wire hover tooltip to textarea |
| `src/styles/global.css` | Add tooltip CSS |
| `src/components/CommandPalette.tsx` | Add `initialQuery` prop |
| `src/App.tsx` | Palette query state, pass to Cell/CommandPalette |

## Verification

1. `npx tsc --noEmit` — compiles
2. Hover over `diff` in a cell → tooltip shows signature and description
3. Hover over non-function word (e.g. `x`) → no tooltip
4. Move mouse away → tooltip disappears
5. Start typing → tooltip disappears
6. Click "Docs →" → command palette opens filtered to that function
7. Works in both light and dark themes
