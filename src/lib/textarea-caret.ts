/**
 * Get pixel coordinates of the caret in a textarea using a mirror div.
 * Returns { top, left } relative to the textarea.
 */
export function getCaretCoordinates(
  textarea: HTMLTextAreaElement,
  position: number
): { top: number; left: number } {
  const mirror = document.createElement("div");
  const style = window.getComputedStyle(textarea);

  // Copy textarea styles to mirror
  const props = [
    "fontFamily",
    "fontSize",
    "fontWeight",
    "fontStyle",
    "letterSpacing",
    "textTransform",
    "wordSpacing",
    "textIndent",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "borderTopWidth",
    "borderRightWidth",
    "borderBottomWidth",
    "borderLeftWidth",
    "boxSizing",
    "lineHeight",
    "tabSize",
  ] as const;

  mirror.style.position = "absolute";
  mirror.style.visibility = "hidden";
  mirror.style.whiteSpace = "pre-wrap";
  mirror.style.wordWrap = "break-word";
  mirror.style.overflow = "hidden";
  mirror.style.width = `${textarea.offsetWidth}px`;

  for (const prop of props) {
    mirror.style[prop] = style[prop];
  }

  document.body.appendChild(mirror);

  const text = textarea.value.substring(0, position);
  mirror.textContent = text;

  // Add a span at the caret position to measure
  const span = document.createElement("span");
  span.textContent = textarea.value.substring(position) || ".";
  mirror.appendChild(span);

  const top = span.offsetTop - textarea.scrollTop;
  const left = span.offsetLeft;

  document.body.removeChild(mirror);

  return { top, left };
}

/**
 * Extract the word being typed at the cursor position.
 * Returns { word, start } where start is the index of the word start.
 */
export function getWordAtCursor(
  text: string,
  cursorPos: number
): { word: string; start: number } {
  let start = cursorPos;
  while (start > 0 && /[a-zA-Z_]/.test(text[start - 1])) {
    start--;
  }
  const word = text.substring(start, cursorPos);
  return { word, start };
}
