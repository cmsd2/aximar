import type { CompletionContext, CompletionResult } from "@codemirror/autocomplete";
import { MATH_SYMBOLS } from "./math-symbols";

/**
 * CodeMirror completion source for LaTeX-style math symbol input.
 * Triggered by typing `\` followed by letters (e.g. `\alpha` → `α`).
 */
export function symbolCompletionSource(
  context: CompletionContext,
): CompletionResult | null {
  const match = context.matchBefore(/\\[a-zA-Z_^0-9]*/);
  if (!match) return null;

  const prefix = match.text.slice(1); // strip leading backslash
  const filtered = prefix
    ? MATH_SYMBOLS.filter((s) => s.latex.startsWith(prefix))
    : MATH_SYMBOLS;

  if (filtered.length === 0) return null;

  return {
    from: match.from,
    filter: false, // we do our own prefix filtering
    options: filtered.map((s) => ({
      label: `\\${s.latex}`,
      displayLabel: `${s.unicode} \\${s.latex}`,
      detail: s.unicode,
      apply: s.unicode,
      type: "text" as const,
    })),
  };
}
