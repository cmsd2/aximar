// The palette's Commands group: the enabled commands whose labels match
// what the user typed
import { COMMANDS, isEnabled, type Command, type CommandState } from "./registry";

/**
 * Commands whose label matches `query`, best first: a label that starts
 * with it, then one whose words start with its words, then one that
 * contains it.  Disabled commands are left out; an empty query matches
 * every enabled command, in list order.
 */
export function matchCommands(
  query: string,
  state: CommandState,
  label: (c: Command) => string,
  limit = Infinity,
): Command[] {
  const q = query.trim().toLowerCase();
  const scored: { c: Command; score: number; i: number }[] = [];
  COMMANDS.forEach((c, i) => {
    if (!isEnabled(c, state)) return;
    const text = label(c).toLowerCase();
    let score = 0;
    if (!q) score = 1;
    else if (text.startsWith(q)) score = 3;
    else if (wordsStart(text, q)) score = 2;
    else if (text.includes(q)) score = 1;
    if (score > 0) scored.push({ c, score, i });
  });
  scored.sort((a, b) => b.score - a.score || a.i - b.i);
  return scored.slice(0, limit).map((s) => s.c);
}

/** Whether each word of `q` starts some word of `text` */
function wordsStart(text: string, q: string): boolean {
  const words = text.split(/[\s.]+/).filter(Boolean);
  return q.split(/\s+/).every((part) => words.some((w) => w.startsWith(part)));
}
