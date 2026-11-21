/** A match of the find query: offsets into the text, in UTF-16 code units. */
export interface TextRange {
  start: number;
  end: number;
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Non-overlapping matches of `query` in `text`, left to right.
 *  Case-insensitive search runs on the original text, so the offsets stay
 *  valid where lower-casing would change a string's length. */
export function findMatches(
  text: string,
  query: string,
  caseSensitive: boolean
): TextRange[] {
  if (!query) return [];
  const re = new RegExp(escapeRegExp(query), caseSensitive ? "gu" : "giu");
  return Array.from(text.matchAll(re), (m) => {
    const start = m.index ?? 0;
    return { start, end: start + m[0].length };
  });
}

/** Replace every non-overlapping match of `query` in `text`. */
export function replaceAllMatches(
  text: string,
  query: string,
  replacement: string,
  caseSensitive: boolean
): string {
  let result = "";
  let last = 0;
  for (const m of findMatches(text, query, caseSensitive)) {
    result += text.slice(last, m.start) + replacement;
    last = m.end;
  }
  return result + text.slice(last);
}
