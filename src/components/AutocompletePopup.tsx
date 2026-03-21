import { useEffect, useRef } from "react";
import type { AutocompleteState } from "../hooks/useAutocomplete";
import { getCaretCoordinates } from "../lib/textarea-caret";

interface AutocompletePopupProps {
  state: AutocompleteState;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  onSelect: (index: number) => void;
  onHover: (index: number) => void;
}

export function AutocompletePopup({
  state,
  textareaRef,
  onSelect,
  onHover,
}: AutocompletePopupProps) {
  const popupRef = useRef<HTMLDivElement>(null);

  // Position the popup relative to the textarea's caret
  useEffect(() => {
    const popup = popupRef.current;
    const textarea = textareaRef.current;
    if (!popup || !textarea || !state.visible) return;

    const coords = getCaretCoordinates(
      textarea,
      state.wordStart
    );
    const rect = textarea.getBoundingClientRect();

    const lineHeight = parseInt(
      window.getComputedStyle(textarea).lineHeight || "20",
      10
    );

    let top = rect.top + coords.top + lineHeight + 4;
    let left = rect.left + coords.left;

    // Keep within viewport
    const popupRect = popup.getBoundingClientRect();
    if (top + popupRect.height > window.innerHeight) {
      top = rect.top + coords.top - popupRect.height - 4;
    }
    if (left + popupRect.width > window.innerWidth) {
      left = window.innerWidth - popupRect.width - 8;
    }

    popup.style.top = `${top}px`;
    popup.style.left = `${left}px`;
  }, [textareaRef, state.visible, state.wordStart]);

  if (!state.visible || state.completions.length === 0) return null;

  return (
    <div className="autocomplete-popup" ref={popupRef}>
      {state.completions.map((c, i) => (
        <div
          key={c.name}
          className={`autocomplete-item ${i === state.selectedIndex ? "selected" : ""}`}
          onMouseDown={(e) => {
            e.preventDefault(); // prevent blur
            onSelect(i);
          }}
          onMouseEnter={() => onHover(i)}
        >
          <span className="autocomplete-name">{c.name}</span>
          <span className="autocomplete-sig">{c.signature}</span>
        </div>
      ))}
    </div>
  );
}
