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

/**
 * Style properties copied from a textarea to a mirror div for accurate
 * text layout measurement.
 */
const MIRROR_PROPS = [
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

/**
 * Given mouse coordinates over a textarea, determine the word at that position.
 * Creates a mirror div positioned over the textarea with each word wrapped in a
 * <span>, then hit-tests the mouse coordinates against span bounding rects.
 *
 * Returns { word, start } or null if no word is found.
 */
export function getWordAtPosition(
  textarea: HTMLTextAreaElement,
  mouseX: number,
  mouseY: number
): { word: string; start: number } | null {
  const rect = textarea.getBoundingClientRect();
  const style = window.getComputedStyle(textarea);

  const mirror = document.createElement("div");
  mirror.style.position = "fixed";
  mirror.style.left = `${rect.left}px`;
  mirror.style.top = `${rect.top}px`;
  mirror.style.width = `${rect.width}px`;
  mirror.style.height = `${rect.height}px`;
  mirror.style.whiteSpace = "pre-wrap";
  mirror.style.wordWrap = "break-word";
  mirror.style.overflow = "hidden";
  mirror.style.visibility = "hidden";

  for (const prop of MIRROR_PROPS) {
    mirror.style[prop] = style[prop];
  }

  // Split text into identifier words and everything else.
  // Wrap each identifier in a <span> so we can measure its bounding rect.
  const text = textarea.value;
  const wordRegex = /[a-zA-Z_][a-zA-Z_0-9]*/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  const wordSpans: { span: HTMLSpanElement; word: string; start: number }[] = [];

  while ((match = wordRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      mirror.appendChild(
        document.createTextNode(text.substring(lastIndex, match.index))
      );
    }
    const span = document.createElement("span");
    span.textContent = match[0];
    mirror.appendChild(span);
    wordSpans.push({ span, word: match[0], start: match.index });
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < text.length) {
    mirror.appendChild(document.createTextNode(text.substring(lastIndex)));
  }

  document.body.appendChild(mirror);
  mirror.scrollTop = textarea.scrollTop;
  mirror.scrollLeft = textarea.scrollLeft;

  let found: { word: string; start: number } | null = null;

  for (const { span, word, start } of wordSpans) {
    const sr = span.getBoundingClientRect();
    if (
      mouseX >= sr.left &&
      mouseX <= sr.right &&
      mouseY >= sr.top &&
      mouseY <= sr.bottom
    ) {
      found = { word, start };
      break;
    }
  }

  document.body.removeChild(mirror);
  return found;
}
