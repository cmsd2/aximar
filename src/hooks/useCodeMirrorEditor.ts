import { useRef, useEffect, useCallback } from "react";
import { EditorState, Compartment } from "@codemirror/state";
import { EditorView, drawSelection, keymap, placeholder as cmPlaceholder, ViewUpdate, tooltips } from "@codemirror/view";
import { defaultKeymap } from "@codemirror/commands";
import { acceptCompletion, autocompletion, completionKeymap } from "@codemirror/autocomplete";
import { maximaLanguage } from "../lib/maxima-language";
import { maximaTheme, maximaHighlightStyle } from "../lib/codemirror-theme";
import { maximaCompletionSource } from "../lib/maxima-completions";
import { symbolCompletionSource } from "../lib/symbol-completions";
import {
  signatureHintField,
  hideSignatureEffect,
  triggerSignatureHint,
  updateSignatureHint,
} from "../lib/maxima-signature-hint";
import { maximaHoverTooltip } from "../lib/maxima-hover-tooltip";
import { findEnclosingCall } from "../lib/param-tracker";
import { useNotebookStore, getActiveTabState } from "../store/notebookStore";
import { useFindStore } from "../store/findStore";

interface UseCodeMirrorEditorOptions {
  cellId: string;
  initialValue: string;
  onExecute: () => void;
  onExecuteStay: () => void;
  onFocusNext: () => void;
  onSetActive: () => void;
  onViewDocs?: (name: string) => void;
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

    // Filter out undo/redo from defaultKeymap
    const filteredKeymap = defaultKeymap.filter(
      (binding) => {
        const key = binding.key?.toLowerCase() ?? "";
        if (key === "mod-z" || key === "mod-y" || key === "mod-shift-z") return false;
        return true;
      }
    );

    const cellKeymap = keymap.of([
      // Swallow undo/redo so notebook-level backend undo works
      // (the keyboard shortcut handler in App.tsx sends these to the backend)
      {
        key: "Mod-z",
        run: () => true,
      },
      {
        key: "Mod-Shift-z",
        run: () => true,
      },
      {
        key: "Mod-y",
        run: () => true,
      },
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
        cmPlaceholder("Enter Maxima expression... (Shift+Enter to evaluate)"),
        autocompleteCompartment.current.of(
          autocompletion({
            override: [symbolCompletionSource, maximaCompletionSource(initialAutocompleteMode)],
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
      ],
    });

    const view = new EditorView({ state, parent: container });
    viewRef.current = view;

    return () => {
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
      const current = view.state.doc.toString();
      if (current !== input) {
        // Preserve cursor position when syncing external content changes
        const prevSel = view.state.selection.main;
        const anchor = Math.min(prevSel.anchor, input.length);
        const head = Math.min(prevSel.head, input.length);
        view.dispatch({
          changes: { from: 0, to: current.length, insert: input },
          selection: { anchor, head },
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
              override: [symbolCompletionSource, maximaCompletionSource(state.autocompleteMode)],
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
