import { useState, useCallback, useRef, useEffect } from "react";
import { getFunction } from "../lib/catalog-client";
import { getWordAtPosition } from "../lib/textarea-caret";
import type { MaximaFunction } from "../types/catalog";

const DEBOUNCE_MS = 150;
const HIDE_DELAY_MS = 300;

export interface HoverTooltipState {
  func: MaximaFunction | null;
  x: number;
  y: number;
  visible: boolean;
}

const initialState: HoverTooltipState = {
  func: null,
  x: 0,
  y: 0,
  visible: false,
};

export function useHoverTooltip(
  textareaRef: React.RefObject<HTMLTextAreaElement | null>,
  autocompleteVisible: boolean
) {
  const [state, setState] = useState<HoverTooltipState>(initialState);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastWordRef = useRef<string>("");
  const cacheRef = useRef<Map<string, MaximaFunction | null>>(new Map());

  const cancelHide = useCallback(() => {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
  }, []);

  const hide = useCallback(() => {
    cancelHide();
    lastWordRef.current = "";
    setState(initialState);
  }, [cancelHide]);

  const scheduleHide = useCallback(() => {
    cancelHide();
    hideTimerRef.current = setTimeout(hide, HIDE_DELAY_MS);
  }, [cancelHide, hide]);

  const onMouseMove = useCallback(
    (e: React.MouseEvent<HTMLTextAreaElement>) => {
      cancelHide();

      if (autocompleteVisible) {
        hide();
        return;
      }

      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }

      const mouseX = e.clientX;
      const mouseY = e.clientY;

      debounceRef.current = setTimeout(() => {
        const textarea = textareaRef.current;
        if (!textarea) return;

        const result = getWordAtPosition(textarea, mouseX, mouseY);
        const word = result?.word ?? "";

        if (!word) {
          hide();
          return;
        }

        // Same word as before — just update position
        if (word === lastWordRef.current && state.visible) {
          return;
        }

        lastWordRef.current = word;

        // Check cache
        if (cacheRef.current.has(word)) {
          const cached = cacheRef.current.get(word) ?? null;
          if (cached) {
            setState({ func: cached, x: mouseX, y: mouseY, visible: true });
          } else {
            setState(initialState);
          }
          return;
        }

        // Look up function
        getFunction(word)
          .then((func) => {
            cacheRef.current.set(word, func);
            // Only show if we're still hovering the same word
            if (lastWordRef.current === word) {
              if (func) {
                setState({ func, x: mouseX, y: mouseY, visible: true });
              } else {
                setState(initialState);
              }
            }
          })
          .catch(() => {
            cacheRef.current.set(word, null);
          });
      }, DEBOUNCE_MS);
    },
    [textareaRef, autocompleteVisible, state.visible, hide]
  );

  const onMouseLeave = useCallback(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    scheduleHide();
  }, [scheduleHide]);

  // Hide tooltip when autocomplete becomes active
  useEffect(() => {
    if (autocompleteVisible) {
      hide();
    }
  }, [autocompleteVisible, hide]);

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
    };
  }, []);

  return { state, onMouseMove, onMouseLeave, hide, cancelHide, scheduleHide };
}
