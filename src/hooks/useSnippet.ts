import { useState, useCallback } from "react";
import { getFunction } from "../lib/catalog-client";
import { parseSignature } from "../lib/signature-parser";

interface Placeholder {
  start: number;
  end: number;
  text: string;
}

export interface SnippetState {
  active: boolean;
  placeholders: Placeholder[];
  currentIndex: number;
}

const INITIAL_STATE: SnippetState = {
  active: false,
  placeholders: [],
  currentIndex: 0,
};

function setTextareaValue(textarea: HTMLTextAreaElement, value: string) {
  const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
    HTMLTextAreaElement.prototype,
    "value"
  )?.set;
  nativeInputValueSetter?.call(textarea, value);
  textarea.dispatchEvent(new Event("input", { bubbles: true }));
}

export function useSnippet(
  textareaRef: React.RefObject<HTMLTextAreaElement | null>
) {
  const [state, setState] = useState<SnippetState>(INITIAL_STATE);

  const activate = useCallback(
    async (funcName: string, insertPos: number) => {
      const textarea = textareaRef.current;
      if (!textarea) return;

      const func = await getFunction(funcName);
      if (!func) return;

      // Find shortest non-zero-param signature
      const parsed = func.signatures.map(parseSignature);
      const withParams = parsed.filter((s) => s.params.length > 0);
      if (withParams.length === 0) return;

      withParams.sort((a, b) => a.params.length - b.params.length);
      const sig = withParams[0];

      // Current text has "name()" inserted. We need to replace the ()
      // with (param1, param2, ...) and create placeholders.
      const text = textarea.value;
      const openParen = insertPos; // position of (
      const closeParen = openParen + 1; // position of )

      const paramText = sig.params.join(", ");
      const newText =
        text.substring(0, openParen + 1) +
        paramText +
        text.substring(closeParen);

      setTextareaValue(textarea, newText);

      // Calculate placeholder positions
      const placeholders: Placeholder[] = [];
      let offset = openParen + 1;
      for (let i = 0; i < sig.params.length; i++) {
        const param = sig.params[i];
        placeholders.push({
          start: offset,
          end: offset + param.length,
          text: param,
        });
        offset += param.length;
        if (i < sig.params.length - 1) {
          offset += 2; // ", "
        }
      }

      setState({
        active: true,
        placeholders,
        currentIndex: 0,
      });

      // Select first placeholder
      if (placeholders.length > 0) {
        textarea.focus();
        textarea.setSelectionRange(placeholders[0].start, placeholders[0].end);
      }
    },
    [textareaRef]
  );

  const exit = useCallback(() => {
    const textarea = textareaRef.current;
    if (textarea && state.active && state.placeholders.length > 0) {
      // Place cursor after the closing paren
      const last = state.placeholders[state.placeholders.length - 1];
      const closeParenPos = last.end + 1; // +1 for the closing paren
      textarea.setSelectionRange(closeParenPos, closeParenPos);
    }
    setState(INITIAL_STATE);
  }, [textareaRef, state]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (!state.active) return false;

      if (e.key === "Tab" && !e.shiftKey) {
        e.preventDefault();
        const nextIndex = state.currentIndex + 1;
        if (nextIndex >= state.placeholders.length) {
          exit();
        } else {
          const textarea = textareaRef.current;
          if (textarea) {
            const ph = state.placeholders[nextIndex];
            textarea.setSelectionRange(ph.start, ph.end);
          }
          setState((s) => ({ ...s, currentIndex: nextIndex }));
        }
        return true;
      }

      if (e.key === "Tab" && e.shiftKey) {
        e.preventDefault();
        const prevIndex = state.currentIndex - 1;
        if (prevIndex >= 0) {
          const textarea = textareaRef.current;
          if (textarea) {
            const ph = state.placeholders[prevIndex];
            textarea.setSelectionRange(ph.start, ph.end);
          }
          setState((s) => ({ ...s, currentIndex: prevIndex }));
        }
        return true;
      }

      if (e.key === "Escape") {
        e.preventDefault();
        exit();
        return true;
      }

      if (e.key === "Enter") {
        // Let Cell handle Enter (Shift+Enter to execute, etc.)
        exit();
        return false;
      }

      return false;
    },
    [state, textareaRef, exit]
  );

  const handleInput = useCallback(() => {
    if (!state.active) return;
    const textarea = textareaRef.current;
    if (!textarea) return;

    const currentPh = state.placeholders[state.currentIndex];
    if (!currentPh) return;

    // Calculate how much the text changed by looking at cursor position
    // relative to the expected placeholder boundaries
    const text = textarea.value;

    // Find where the current placeholder content now ends
    // The user is typing in place of the placeholder, so we recalculate
    // The new end of this placeholder region is the cursor position
    // (assuming they're typing forward from the start)
    const oldLength = currentPh.end - currentPh.start;
    // Estimate new length: from placeholder start to before the next delimiter
    // Find next comma or close paren after placeholder start
    let newEnd = currentPh.start;
    let depth = 0;
    for (let i = currentPh.start; i < text.length; i++) {
      const ch = text[i];
      if (ch === "(" || ch === "[") depth++;
      else if (ch === ")" || ch === "]") {
        if (depth === 0) { newEnd = i; break; }
        depth--;
      } else if (ch === "," && depth === 0) {
        newEnd = i;
        break;
      }
      if (i === text.length - 1) newEnd = text.length;
    }

    const newLength = newEnd - currentPh.start;
    const delta = newLength - oldLength;

    if (delta === 0) return;

    // Update placeholder positions
    setState((s) => {
      const newPlaceholders = s.placeholders.map((ph, i) => {
        if (i === s.currentIndex) {
          return { ...ph, end: ph.start + newLength, text: text.substring(ph.start, ph.start + newLength) };
        }
        if (i > s.currentIndex) {
          return { ...ph, start: ph.start + delta, end: ph.end + delta };
        }
        return ph;
      });
      return { ...s, placeholders: newPlaceholders };
    });
  }, [state, textareaRef]);

  return { state, activate, exit, handleKeyDown, handleInput };
}
