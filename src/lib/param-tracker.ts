/**
 * Given text, the position of the opening paren, and the cursor position,
 * return which parameter index (0-based) the cursor is on.
 * Returns null if the cursor is past the closing paren.
 */
export function getParamIndex(
  text: string,
  openParenPos: number,
  cursorPos: number
): number | null {
  let depth = 0;
  let paramIndex = 0;
  let inString = false;

  for (let i = openParenPos + 1; i < cursorPos && i < text.length; i++) {
    const ch = text[i];

    if (inString) {
      if (ch === '"' && text[i - 1] !== "\\") {
        inString = false;
      }
      continue;
    }

    if (ch === '"') {
      inString = true;
      continue;
    }

    if (ch === "(" || ch === "[") {
      depth++;
    } else if (ch === ")" || ch === "]") {
      if (depth === 0) {
        // Cursor is past closing paren
        return null;
      }
      depth--;
    } else if (ch === "," && depth === 0) {
      paramIndex++;
    }
  }

  // Check if cursor is actually past the closing paren
  // by scanning from openParenPos to find the matching close
  let checkDepth = 0;
  for (let i = openParenPos + 1; i < text.length; i++) {
    const ch = text[i];
    if (ch === "(" || ch === "[") checkDepth++;
    else if (ch === ")" || ch === "]") {
      if (checkDepth === 0) {
        // Found the matching close paren
        if (cursorPos > i) return null;
        break;
      }
      checkDepth--;
    }
  }

  return paramIndex;
}

/**
 * Scan backwards from cursor to find the innermost enclosing function call.
 * Returns the function name and the position of its opening paren, or null.
 */
export function findEnclosingCall(
  text: string,
  cursorPos: number
): { funcName: string; openParenPos: number } | null {
  let depth = 0;

  for (let i = cursorPos - 1; i >= 0; i--) {
    const ch = text[i];

    if (ch === ")" || ch === "]") {
      depth++;
    } else if (ch === "[") {
      if (depth === 0) {
        // Inside brackets, not a function call
        return null;
      }
      depth--;
    } else if (ch === "(") {
      if (depth === 0) {
        // Found unmatched opening paren — extract preceding identifier
        let nameEnd = i;
        let nameStart = i;
        while (nameStart > 0 && /[a-zA-Z_0-9]/.test(text[nameStart - 1])) {
          nameStart--;
        }
        const funcName = text.substring(nameStart, nameEnd);
        if (funcName && /^[a-zA-Z_]/.test(funcName)) {
          return { funcName, openParenPos: i };
        }
        // Bare parens without identifier — not a function call
        return null;
      }
      depth--;
    }
  }

  return null;
}
