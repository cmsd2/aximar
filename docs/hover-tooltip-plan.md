# Function Hover Tooltip

> **Status**: Fully implemented (CodeMirror 6).

## Context

Users can hover over function names in code cells to see what they do, with the option to jump to full documentation. Implemented using CodeMirror 6's `hoverTooltip()` API, which provides built-in word-at-position detection and tooltip positioning.

The `getFunction(name)` backend command returns function metadata (signatures, description, category, examples, see_also) from the catalog.

## Implementation

### `src/lib/maxima-hover-tooltip.ts` — CM6 hover tooltip extension

Uses CM's `hoverTooltip()` factory:
- On hover, finds the word at the cursor position
- Looks up the function via `getFunction(name)` Tauri IPC (with a local cache)
- Renders tooltip DOM with signatures, description, category, and "Docs" button
- `hideOnChange: true` dismisses tooltip on typing

### `src/hooks/useCodeMirrorEditor.ts` — Integration

The hover tooltip extension is added to the CM6 extension stack. The `onViewDocs` callback is passed through via a ref so it stays current without recreating the extension.

Tooltips are rendered into `document.body` via `tooltips({ parent: document.body })` to avoid clipping by cell overflow boundaries.

### `src/styles/global.css` — Tooltip styles

- `.hover-tooltip`: `max-width: 320px`, padding, pointer-events
- Sub-elements for signature, description, category, "Docs" link
- Positioned by CM's tooltip system (no `position: fixed` needed)

### `src/components/CommandPalette.tsx` — Accept `initialQuery` prop

- Optional `initialQuery?: string` prop, used as the starting `query` state
- "Docs" button in hover tooltip opens the command palette filtered to that function

## Files

| File | Action |
|---|---|
| `src/lib/maxima-hover-tooltip.ts` | CM6 hover tooltip extension |
| `src/hooks/useCodeMirrorEditor.ts` | Add hover tooltip to extension stack |
| `src/styles/global.css` | Tooltip CSS |
| `src/components/CommandPalette.tsx` | `initialQuery` prop |
| `src/App.tsx` | Palette query state, pass to Cell/CommandPalette |

## Verification

1. `npx tsc --noEmit` — compiles
2. Hover over `diff` in a cell — tooltip shows signature and description
3. Hover over non-function word (e.g. `x`) — no tooltip
4. Move mouse away — tooltip disappears
5. Start typing — tooltip disappears
6. Click "Docs" — command palette opens filtered to that function
7. Works in both light and dark themes
