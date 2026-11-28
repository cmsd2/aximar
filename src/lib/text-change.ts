/** One replacement in a text: the range `from`..`to` becomes `insert` */
export interface TextChange {
  from: number;
  to: number;
  insert: string;
}

function isHighSurrogate(code: number): boolean {
  return code >= 0xd800 && code <= 0xdbff;
}

function isLowSurrogate(code: number): boolean {
  return code >= 0xdc00 && code <= 0xdfff;
}

/**
 * The one replacement that turns `current` into `next`, keeping the text
 * they share at the start and end, or null when they are the same.  Never
 * splits a character written as a surrogate pair.
 */
export function changeBetween(current: string, next: string): TextChange | null {
  if (current === next) return null;
  const shorter = Math.min(current.length, next.length);
  let start = 0;
  while (start < shorter && current.charCodeAt(start) === next.charCodeAt(start)) start++;
  if (start > 0 && isHighSurrogate(current.charCodeAt(start - 1))) start--;

  let endCurrent = current.length;
  let endNext = next.length;
  while (
    endCurrent > start &&
    endNext > start &&
    current.charCodeAt(endCurrent - 1) === next.charCodeAt(endNext - 1)
  ) {
    endCurrent--;
    endNext--;
  }
  if (endCurrent < current.length && isLowSurrogate(current.charCodeAt(endCurrent))) {
    endCurrent++;
    endNext++;
  }
  return { from: start, to: endCurrent, insert: next.slice(start, endNext) };
}
