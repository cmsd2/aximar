# Frontend Separation of Concerns

Audit performed 2026-03-24 against `master` at commit `8d6f1c3`.

The frontend is a React + Zustand app that should be primarily presentational, with business logic and backend communication handled by hooks and utility modules. Several files currently violate this principle.

## Critical

### 1. App.tsx (400 lines) — god component

`App.tsx` orchestrates config loading, session initialisation, menu events, keyboard shortcuts, file operations, log buffering, and close-window guards all in one component.

**Specific issues:**
- 6+ `useEffect` blocks handling unrelated backend sync concerns
- Config loading + DOM manipulation (CSS variables, dataset attributes)
- Welcome notebook loading logic
- Menu event routing (~20 lines of switch/case)
- Close-window guard with dialog interaction
- Keyboard shortcut handling (~60 lines)

**Suggested refactoring:**
- `useKeyboardShortcuts()` — extract shortcut handling
- `useConfigLoader()` — extract config loading and DOM theme application
- `useFileOperations()` — extract save/open/new notebook logic
- `useMenuEvents()` — extract Tauri menu event listener

### 2. SettingsModal.tsx (489 lines) — orchestration in a component

Directly calls `getConfig()`, `setConfig()`, `listWslDistros()`, `checkWslMaxima()`. Contains a monolithic 60-line `update()` callback that handles theme, fonts, margins, and backend config. Manipulates the DOM (CSS variables, dataset attributes).

**Suggested refactoring:**
- Extract WSL-specific logic to a separate component or hook
- Extract theme/styling application to `useThemeSettings()`
- Extract config persistence to `useConfigPersistence()`
- Split the render into smaller sub-components by settings category

### 3. useCodeMirrorEditor.ts (310 lines) — hook doing too much

Combines editor creation with signature hint logic, autocomplete mode switching, find-replace sync, and cursor movement coordination with the store.

**Suggested refactoring:**
- Extract signature hint logic to `useSignatureHints()`
- Extract completion mode handling to `useCompletionMode()`
- Extract find-replace sync (move to FindBar or separate hook)
- Keep only editor creation, view destruction, and external input sync

### 4. DocsPanel.tsx (317 lines) — search + navigation + rendering

Implements search debouncing, browser-like history navigation (manual refs + forceUpdate hack), KaTeX rendering, and backend data fetching all in one component.

**Suggested refactoring:**
- `useDocsSearch()` — extract search debouncing
- `useDocsHistory()` — extract navigation history
- `<KatexCodeBlock>` — extract KaTeX rendering to a component
- `docMarkdownComponents()` — extract markdown customisation to a utility

## High

### 5. CommandPalette.tsx (359 lines) — multiple concerns

Contains function insertion logic, a category selection state machine, and 50+ lines of keyboard event handling. Calls `listCategories()` and `searchFunctions()` directly.

**Suggested refactoring:**
- `useCommandInsert()` — extract insertion logic
- `useCommandMode()` — extract mode/category selection
- `useCommandKeyboard()` — extract keyboard handling

### 6. FindBar.tsx (227 lines) — algorithms in a component

Implements the full find/replace search algorithm and text replacement logic inside the component.

**Suggested refactoring:**
- Extract find/replace algorithms to `src/lib/find-replace.ts`
- Extract UI coordination to `useFindReplace()` hook

### 7. CellSuggestions.tsx (103 lines) — orchestration in a component

Direct Tauri `invoke()` calls, cell reuse heuristics, and execution orchestration.

**Suggested refactoring:**
- Extract suggestion action handling to `useSuggestionHandler()`
- Extract cell reuse heuristic to a utility function

### 8. useMaxima.ts (121 lines) — mixed concerns

Combines platform detection, installation dialog UI, session management, and cell execution in one hook.

**Suggested refactoring:**
- Extract dialog logic to `useMaximaNotFoundDialog()`
- Extract platform detection to a `getPlatform()` utility
- Consider splitting session management from cell execution

### 9. useNotebookEvents.ts (157 lines) — dual-direction sync

Both listens for backend state events and manages debounced input sync to the backend. Also contains DOM focus manipulation.

**Suggested refactoring:**
- Extract DOM focus logic to `useAutoFocus()` or a utility
- Extract debounced input sync to `useInputSync()`
- Keep only backend state event listening

## Summary

| File | Lines | Priority | Primary issues |
|------|-------|----------|----------------|
| App.tsx | 400 | Critical | Orchestration, config, sessions, events, shortcuts |
| SettingsModal.tsx | 489 | Critical | Backend calls, DOM manipulation, monolithic update |
| useCodeMirrorEditor.ts | 310 | Critical | Editor + signatures + completions + find sync |
| DocsPanel.tsx | 317 | Critical | Search, navigation, rendering, KaTeX |
| CommandPalette.tsx | 359 | High | Insertion, state machine, keyboard, catalog |
| FindBar.tsx | 227 | High | Find/replace algorithms in component |
| CellSuggestions.tsx | 103 | High | Tauri calls, cell reuse, execution |
| useMaxima.ts | 121 | High | Platform detection, dialogs, sessions, execution |
| useNotebookEvents.ts | 157 | High | Dual-direction sync, debounce, DOM focus |

## Refactoring approach

**Phase 1 — App.tsx:** Extract focused hooks. This is the highest-impact change since every concern in App.tsx is independent and easily separable.

**Phase 2 — Large components:** Split SettingsModal, CommandPalette, and DocsPanel into smaller presentational components with logic extracted to hooks.

**Phase 3 — Hooks:** Split useCodeMirrorEditor and useNotebookEvents by concern.

**Phase 4 — Utilities:** Move algorithms (find/replace, cell reuse, platform detection) to `src/lib/` modules.
