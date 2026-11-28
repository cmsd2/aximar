import { useRef, useEffect, useCallback } from "react";
import { EditorState, Compartment } from "@codemirror/state";
import { EditorView, drawSelection, keymap, placeholder as cmPlaceholder, ViewUpdate, tooltips } from "@codemirror/view";
import { defaultKeymap } from "@codemirror/commands";
import { acceptCompletion, autocompletion, completionKeymap } from "@codemirror/autocomplete";
import { i18n } from "@lingui/core";
import { t } from "@lingui/core/macro";
import { maximaLanguage } from "../lib/maxima-language";
import { maximaTheme, maximaHighlightStyle } from "../lib/codemirror-theme";
import { maximaCompletionSource } from "../lib/maxima-completions";
import { symbolCompletionSource } from "../lib/symbol-completions";
import { drawContextCompletionSource } from "../lib/draw-completions";
import {
  signatureHintField,
  hideSignatureEffect,
  triggerSignatureHint,
  updateSignatureHint,
} from "../lib/maxima-signature-hint";
import { maximaHoverTooltip } from "../lib/maxima-hover-tooltip";
import { gapBeside } from "../lib/cell-caret";
import { findEnclosingCall } from "../lib/param-tracker";
import { changeBetween } from "../lib/text-change";
import { undoOrRedo } from "../commands/registry";
import { useNotebookStore, getActiveTabState } from "../store/notebookStore";
import { useFindStore } from "../store/findStore";

/** The placeholder of an empty code cell, in the window's language */
function placeholderExtension() {
  return cmPlaceholder(
    t({
      message: "Enter Maxima expression... (Shift+Enter to evaluate)",
      comment: "Placeholder in an empty code cell. Shift+Enter is the keyboard shortcut; keep it as it is",
    }),
  );
}

interface UseCodeMirrorEditorOptions {
  cellId: string;
  initialValue: string;
  onExecute: () => void;
  onExecuteStay: () => void;
  onFocusNext: () => void;
  onSetActive: () => void;
  onViewDocs?: (name: string) => void;
}


/**
 * CodeMirror's default keys, less those a cell's editor gives up: undo and
 * redo, which are the notebook's; the add-cursor keys, which Move Cell Up
 * and Down use; and toggle-comment, which Help > Keyboard Shortcuts uses and
 * which would do nothing here anyway, as the Maxima language declares no
 * comment syntax (src/commands/registry.ts).  A key the editor binds never
 * reaches the menu or the window, so no menu shortcut may be one of its
 * keys; editor-keys.test.ts checks.
 */
export function cellEditorKeymap(): typeof defaultKeymap {
  const freed = new Set(["mod-z", "mod-y", "mod-shift-z", "mod-alt-arrowup", "mod-alt-arrowdown", "mod-/"]);
  return defaultKeymap.filter((binding) => !freed.has(binding.key?.toLowerCase() ?? ""));
}

export function useCodeMirrorEditor({
  cellId,
  initialValue,
  onExecute,
  onExecuteStay,
  onFocusNext,
  onSetActive,
  onViewDocs,
}: UseCodeMirrorEditorOptions) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const isInternalUpdate = useRef(false);
  const autocompleteCompartment = useRef(new Compartment());
  const placeholderCompartment = useRef(new Compartment());

  // Stable refs for callbacks to avoid recreating extensions
  const onExecuteRef = useRef(onExecute);
  const onExecuteStayRef = useRef(onExecuteStay);
  const onFocusNextRef = useRef(onFocusNext);
  const onSetActiveRef = useRef(onSetActive);
  const onViewDocsRef = useRef(onViewDocs);

  useEffect(() => { onExecuteRef.current = onExecute; }, [onExecute]);
  useEffect(() => { onExecuteStayRef.current = onExecuteStay; }, [onExecuteStay]);
  useEffect(() => { onFocusNextRef.current = onFocusNext; }, [onFocusNext]);
  useEffect(() => { onSetActiveRef.current = onSetActive; }, [onSetActive]);
  useEffect(() => { onViewDocsRef.current = onViewDocs; }, [onViewDocs]);

  // Create editor on mount
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const filteredKeymap = cellEditorKeymap();

    const cellKeymap = keymap.of([
      // Undo and redo are the notebook's (src/commands/registry.ts); the
      // editor keeps no history of its own.  It runs them for their keys
      // itself: on Windows the keys reach the page, not the menu, and
      // left alone the browser ran its own undo, which knows only the
      // typing it saw and moves the cursor under CodeMirror's feet.
      { key: "Mod-z", preventDefault: true, run: () => (undoOrRedo("undo", "editor"), true) },
      { key: "Mod-y", preventDefault: true, run: () => (undoOrRedo("redo", "editor"), true) },
      { key: "Mod-Shift-z", preventDefault: true, run: () => (undoOrRedo("redo", "editor"), true) },
      // Cell execution — dismiss signature hint before executing
      {
        key: "Shift-Enter",
        run: (view) => {
          view.dispatch({ effects: hideSignatureEffect.of(undefined) });
          onExecuteRef.current();
          return true;
        },
      },
      {
        key: "Mod-Enter",
        run: (view) => {
          view.dispatch({ effects: hideSignatureEffect.of(undefined) });
          onExecuteStayRef.current();
          return true;
        },
      },
      // From the first or last line, the arrow keys go to the caret in the
      // gap above or below the cell (see CellGap)
      { key: "ArrowUp", run: (view) => toGap(view, cellId, "up") },
      { key: "ArrowDown", run: (view) => toGap(view, cellId, "down") },
      // Escape to dismiss signature hint or blur
      {
        key: "Escape",
        run: (view) => {
          const hasHint = view.state.field(signatureHintField);
          if (hasHint) {
            view.dispatch({ effects: hideSignatureEffect.of(undefined) });
            return true;
          }
          view.contentDOM.blur();
          return true;
        },
      },
    ]);

    // Track previous doc to detect "(" typed
    let prevDoc = initialValue;

    const updateListener = EditorView.updateListener.of((update: ViewUpdate) => {
      if (update.docChanged) {
        isInternalUpdate.current = true;
        const newText = update.state.doc.toString();
        useNotebookStore.getState().updateCellInput(cellId, newText);
        isInternalUpdate.current = false;

        // Detect "(" typed → trigger signature hint
        const autocompleteMode = useNotebookStore.getState().autocompleteMode;
        const cursorPos = update.state.selection.main.head;
        if (
          newText.length > prevDoc.length &&
          cursorPos > 0 &&
          newText[cursorPos - 1] === "("
        ) {
          const hasHint = update.state.field(signatureHintField);
          if (!hasHint && autocompleteMode !== "snippet") {
            const call = findEnclosingCall(newText, cursorPos);
            if (call) {
              triggerSignatureHint(update.view, call.funcName, call.openParenPos, autocompleteMode);
            }
          }
        }

        // Dismiss hint on ")" typed in hint mode
        if (autocompleteMode === "hint") {
          const hasHint = update.state.field(signatureHintField);
          if (hasHint && cursorPos > 0 && newText[cursorPos - 1] === ")") {
            update.view.dispatch({ effects: hideSignatureEffect.of(undefined) });
          }
        }

        // Update active param index in active-hint mode
        if (autocompleteMode === "active-hint") {
          const hasHint = update.state.field(signatureHintField);
          if (hasHint) {
            updateSignatureHint(update.view);
          }
        }

        prevDoc = newText;
      }

      if (update.focusChanged) {
        if (update.view.hasFocus) {
          onSetActiveRef.current();
        } else {
          // Dismiss signature hint on blur
          setTimeout(() => {
            const view = viewRef.current;
            if (view && !view.hasFocus) {
              view.dispatch({ effects: hideSignatureEffect.of(undefined) });
            }
          }, 150);
        }
      }
    });

    // The browser's undo by any other way, such as its context menu,
    // runs the notebook's instead
    const browserUndo = EditorView.domEventHandlers({
      beforeinput: (event) => {
        const which = event.inputType === "historyUndo" ? "undo" : event.inputType === "historyRedo" ? "redo" : null;
        if (!which) return false;
        event.preventDefault();
        undoOrRedo(which, "editor");
        return true;
      },
    });

    const initialAutocompleteMode = useNotebookStore.getState().autocompleteMode;

    const state = EditorState.create({
      doc: initialValue,
      extensions: [
        cellKeymap,
        keymap.of([{ key: "Tab", run: acceptCompletion }, ...completionKeymap, ...filteredKeymap]),
        maximaLanguage,
        maximaTheme,
        maximaHighlightStyle,
        drawSelection(),
        EditorView.lineWrapping,
        placeholderCompartment.current.of(placeholderExtension()),
        autocompleteCompartment.current.of(
          autocompletion({
            override: [drawContextCompletionSource, symbolCompletionSource, maximaCompletionSource(initialAutocompleteMode)],
            activateOnTyping: true,
            maxRenderedOptions: 8,
          })
        ),
        tooltips({ parent: document.body }),
        signatureHintField,
        maximaHoverTooltip((name: string) => {
          onViewDocsRef.current?.(name);
        }),
        updateListener,
        browserUndo,
      ],
    });

    const view = new EditorView({ state, parent: container });
    viewRef.current = view;

    // The placeholder follows the language when it changes
    const stopListening = i18n.on("change", () => {
      view.dispatch({
        effects: placeholderCompartment.current.reconfigure(placeholderExtension()),
      });
    });

    return () => {
      stopListening();
      view.destroy();
      viewRef.current = null;
    };
    // Only run on mount/unmount — cellId is stable for the lifetime of a cell
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cellId]);

  // Sync external changes (undo/redo, find-replace) into CM,
  // then apply any pending cursor move or find navigation.
  const syncExternalInput = useCallback(
    (input: string) => {
      const view = viewRef.current;
      if (!view || isInternalUpdate.current) return;
      // Replace only the text that changed, and put the cursor at the end
      // of it, as an editor's own undo does.  Keeping the cursor's offset
      // in the whole text put it a character or a line away from an undone
      // change made before it.
      const change = changeBetween(view.state.doc.toString(), input);
      if (change) {
        view.dispatch({
          changes: change,
          selection: { anchor: change.from + change.insert.length },
          scrollIntoView: view.hasFocus,
        });
      }

      // Apply pending cursor move (e.g. from command palette insert)
      const move = getActiveTabState().pendingCursorMove;
      if (move && move.cellId === cellId) {
        const pos = Math.min(move.pos, view.state.doc.length);
        view.focus();
        view.dispatch({
          selection: { anchor: pos },
          scrollIntoView: true,
        });
        useNotebookStore.getState().clearPendingCursorMove();

        // If cursor lands inside a function call, trigger signature hint
        const text = view.state.doc.toString();
        const autocompleteMode = useNotebookStore.getState().autocompleteMode;
        const call = findEnclosingCall(text, pos);
        if (call && autocompleteMode !== "snippet") {
          triggerSignatureHint(view, call.funcName, call.openParenPos, autocompleteMode);
        }
      }

      // Apply pending find navigation
      const nav = useFindStore.getState().navigateTo;
      if (nav && nav.cellId === cellId) {
        view.focus();
        view.dispatch({
          selection: { anchor: nav.start, head: nav.end },
          scrollIntoView: true,
        });
        useFindStore.getState().clearNavigateTo();
      }
    },
    [cellId]
  );

  // Watch store for find navigateTo (fallback for when content hasn't changed)
  useEffect(() => {
    return useFindStore.subscribe((state) => {
      const nav = state.navigateTo;
      if (!nav || nav.cellId !== cellId) return;
      const view = viewRef.current;
      if (!view) return;
      view.focus();
      view.dispatch({
        selection: { anchor: nav.start, head: nav.end },
        scrollIntoView: true,
      });
      useFindStore.getState().clearNavigateTo();
    });
  }, [cellId]);

  // A cursor move asked for after this editor mounted (a new cell taking
  // typed text, an arrow key from a gap)
  useEffect(() => {
    return useNotebookStore.subscribe(() => {
      const move = getActiveTabState().pendingCursorMove;
      const view = viewRef.current;
      if (!move || move.cellId !== cellId || !view) return;
      view.focus();
      view.dispatch({ selection: { anchor: Math.min(move.pos, view.state.doc.length) }, scrollIntoView: true });
      useNotebookStore.getState().clearPendingCursorMove();
    });
  }, [cellId]);

  // Reconfigure autocomplete when mode changes
  useEffect(() => {
    return useNotebookStore.subscribe((state, prev) => {
      if (state.autocompleteMode === prev.autocompleteMode) return;
      const view = viewRef.current;
      if (!view) return;
      // Dismiss signature hint on mode change
      view.dispatch({
        effects: [
          hideSignatureEffect.of(undefined),
          autocompleteCompartment.current.reconfigure(
            autocompletion({
              override: [drawContextCompletionSource, symbolCompletionSource, maximaCompletionSource(state.autocompleteMode)],
              activateOnTyping: true,
              maxRenderedOptions: 8,
            })
          ),
        ],
      });
    });
  }, []);

  return {
    containerRef,
    view: viewRef,
    syncExternalInput,
  };
}

/**
 * Move to the gap above or below the cell when the cursor is on the
 * editor's first or last visual line with nothing selected.  Returns false
 * to let CodeMirror move the cursor otherwise.
 */
function toGap(view: EditorView, cellId: string, dir: "up" | "down"): boolean {
  const sel = view.state.selection.main;
  if (!sel.empty) return false;
  const moved = view.moveVertically(sel, dir === "down");
  const here = view.coordsAtPos(sel.head);
  const there = view.coordsAtPos(moved.head);
  const line = view.state.doc.lineAt(sel.head).number;
  const atEdge =
    here && there
      ? Math.abs(here.top - there.top) < 1
      : dir === "up"
        ? line === 1
        : line === view.state.doc.lines;
  if (!atEdge) return false;
  const point = gapBeside(getActiveTabState().cells, cellId, dir);
  if (!point) return false;
  view.contentDOM.blur();
  useNotebookStore.getState().setInsertionPoint(point);
  return true;
}
