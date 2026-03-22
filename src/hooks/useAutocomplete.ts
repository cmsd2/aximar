import { useState, useCallback, useRef, useEffect } from "react";
import { completeFunction } from "../lib/catalog-client";
import { getWordAtCursor } from "../lib/textarea-caret";
import type { CompletionResult } from "../types/catalog";

const MIN_PREFIX_LENGTH = 2;
const DEBOUNCE_MS = 100;

export interface AutocompleteState {
  completions: CompletionResult[];
  selectedIndex: number;
  visible: boolean;
  prefix: string;
  wordStart: number;
}

export interface AcceptResult {
  funcName: string;
  insertPosition: number;
}

export function useAutocomplete(
  textareaRef: React.RefObject<HTMLTextAreaElement | null>,
  onAccept?: (result: AcceptResult) => void
) {
  const [state, setState] = useState<AutocompleteState>({
    completions: [],
    selectedIndex: 0,
    visible: false,
    prefix: "",
    wordStart: 0,
  });

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const dismiss = useCallback(() => {
    setState((s) => ({ ...s, visible: false, completions: [] }));
  }, []);

  const fetchCompletions = useCallback((prefix: string, wordStart: number) => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }

    if (prefix.length < MIN_PREFIX_LENGTH) {
      setState((s) => ({ ...s, visible: false, completions: [] }));
      return;
    }

    debounceRef.current = setTimeout(() => {
      completeFunction(prefix)
        .then((results) => {
          if (results.length > 0) {
            setState({
              completions: results.slice(0, 8),
              selectedIndex: 0,
              visible: true,
              prefix,
              wordStart,
            });
          } else {
            setState((s) => ({ ...s, visible: false, completions: [] }));
          }
        })
        .catch(() => {});
    }, DEBOUNCE_MS);
  }, []);

  const accept = useCallback((): AcceptResult | false => {
    const textarea = textareaRef.current;
    if (!textarea || !state.visible || state.completions.length === 0) return false;

    const completion = state.completions[state.selectedIndex];
    const text = textarea.value;
    const before = text.substring(0, state.wordStart);
    const after = text.substring(state.wordStart + state.prefix.length);
    const insertText = completion.insert_text;

    // Update the textarea value
    const newValue = before + insertText + after;
    const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
      HTMLTextAreaElement.prototype,
      "value"
    )?.set;
    nativeInputValueSetter?.call(textarea, newValue);
    textarea.dispatchEvent(new Event("input", { bubbles: true }));

    // Position cursor inside the parentheses
    const cursorPos = before.length + insertText.length - 1;
    textarea.setSelectionRange(cursorPos, cursorPos);

    const insertPosition = before.length + completion.name.length;
    const result: AcceptResult = { funcName: completion.name, insertPosition };

    dismiss();
    onAccept?.(result);
    return result;
  }, [textareaRef, state, dismiss, onAccept]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (!state.visible) return false;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        setState((s) => ({
          ...s,
          selectedIndex: Math.min(s.selectedIndex + 1, s.completions.length - 1),
        }));
        return true;
      }

      if (e.key === "ArrowUp") {
        e.preventDefault();
        setState((s) => ({
          ...s,
          selectedIndex: Math.max(s.selectedIndex - 1, 0),
        }));
        return true;
      }

      if (e.key === "Tab" || e.key === "Enter") {
        if (state.visible && state.completions.length > 0) {
          e.preventDefault();
          accept();
          return true;
        }
      }

      if (e.key === "Escape") {
        e.preventDefault();
        dismiss();
        return true;
      }

      return false;
    },
    [state.visible, state.completions.length, accept, dismiss]
  );

  const handleInput = useCallback(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    const cursorPos = textarea.selectionStart;
    const { word, start } = getWordAtCursor(textarea.value, cursorPos);
    fetchCompletions(word, start);
  }, [textareaRef, fetchCompletions]);

  // Cleanup debounce on unmount
  useEffect(() => {
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
    };
  }, []);

  return {
    state,
    handleKeyDown,
    handleInput,
    accept,
    dismiss,
  };
}
