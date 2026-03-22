import { useEffect, useRef } from "react";
import { getCaretCoordinates } from "../lib/textarea-caret";
import type { SignatureHintState } from "../hooks/useSignatureHint";

const MAX_SHOWN = 5;

interface SignatureHintProps {
  state: SignatureHintState;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
}

export function SignatureHint({ state, textareaRef }: SignatureHintProps) {
  const popupRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const popup = popupRef.current;
    const textarea = textareaRef.current;
    if (!popup || !textarea || !state.visible) return;

    const coords = getCaretCoordinates(textarea, state.anchorPosition);
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
  }, [textareaRef, state.visible, state.anchorPosition, state.activeParamIndex]);

  if (!state.visible || state.signatures.length === 0) return null;

  const shown = state.signatures.slice(0, MAX_SHOWN);
  const overflow = state.signatures.length - MAX_SHOWN;

  return (
    <div className="signature-hint" ref={popupRef}>
      {shown.map((sig, i) => {
        const exceeded =
          state.activeParamIndex !== null &&
          sig.params.length > 0 &&
          state.activeParamIndex >= sig.params.length;

        return (
          <div
            key={i}
            className={`signature-hint-line${exceeded ? " sig-hint-overflow" : ""}`}
          >
            <span className="signature-hint-name">{sig.name}</span>
            <span className="signature-hint-parens">(</span>
            {sig.params.map((param, pi) => (
              <span key={pi}>
                {pi > 0 && <span className="signature-hint-comma">, </span>}
                <span
                  className={
                    state.activeParamIndex === pi
                      ? "sig-hint-active"
                      : "signature-hint-param"
                  }
                >
                  {param}
                </span>
              </span>
            ))}
            <span className="signature-hint-parens">)</span>
          </div>
        );
      })}
      {overflow > 0 && (
        <div className="signature-hint-more">+{overflow} more</div>
      )}
    </div>
  );
}
