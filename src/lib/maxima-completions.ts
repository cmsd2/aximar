import type { CompletionContext, CompletionResult } from "@codemirror/autocomplete";
import { snippet } from "@codemirror/autocomplete";
import { completeFunction, completePackages, getFunction } from "./catalog-client";
import { parseSignature } from "./signature-parser";

export function maximaCompletionSource(autocompleteMode: string) {
  return async (context: CompletionContext): Promise<CompletionResult | null> => {
    // Check if we're inside a load("...") call first
    const loadResult = await loadPackageCompletionSource(context);
    if (loadResult) return loadResult;

    const word = context.matchBefore(/[a-zA-Z_][a-zA-Z_0-9]*/);
    if (!word || word.text.length < 2) return null;

    const results = await completeFunction(word.text);
    if (results.length === 0) return null;

    return {
      from: word.from,
      options: results.slice(0, 8).map((r) => ({
        label: r.name,
        detail: r.package ? `load("${r.package}")` : r.signature,
        info: r.description || undefined,
        type: "function" as const,
        apply: autocompleteMode === "snippet"
          ? createSnippetApply(r.name)
          : `${r.name}(`,
      })),
    };
  };
}

/**
 * Detect when cursor is inside load("...") and offer package name completions.
 */
async function loadPackageCompletionSource(
  context: CompletionContext
): Promise<CompletionResult | null> {
  // Match load("prefix or load(prefix (with or without quotes)
  const match = context.matchBefore(/load\s*\(\s*"?([a-zA-Z_/]*)/);
  if (!match) return null;

  // Extract the prefix (the part after the opening quote/paren)
  const fullMatch = match.text;
  const prefixMatch = fullMatch.match(/load\s*\(\s*"?([a-zA-Z_/]*)$/);
  if (!prefixMatch) return null;

  const prefix = prefixMatch[1];
  const prefixStart = match.to - prefix.length;

  const results = await completePackages(prefix);
  if (results.length === 0) return null;

  // Check if there's already a closing quote and paren after cursor
  const after = context.state.sliceDoc(context.pos, context.pos + 10);
  const hasClosingQuote = after.startsWith('"');
  const hasQuoteAndParen = after.startsWith('")');

  return {
    from: prefixStart,
    options: results.slice(0, 10).map((r) => ({
      label: r.name,
      info: r.description || undefined,
      type: "keyword" as const,
      apply: hasQuoteAndParen
        ? r.name
        : hasClosingQuote
          ? r.name
          : `${r.name}`,
    })),
  };
}

function createSnippetApply(funcName: string) {
  // Returns a function that fetches the signature and creates a snippet
  return async (_view: import("@codemirror/view").EditorView, completion: import("@codemirror/autocomplete").Completion, from: number, to: number) => {
    const func = await getFunction(funcName);
    if (!func) {
      // Fallback: just insert name(
      _view.dispatch({
        changes: { from, to, insert: `${funcName}(` },
        selection: { anchor: from + funcName.length + 1 },
      });
      return;
    }

    const parsed = func.signatures.map(parseSignature);
    const withParams = parsed.filter((s) => s.params.length > 0);

    if (withParams.length === 0) {
      // No params, just insert name()
      _view.dispatch({
        changes: { from, to, insert: `${funcName}()` },
        selection: { anchor: from + funcName.length + 2 },
      });
      return;
    }

    // Use shortest signature
    withParams.sort((a, b) => a.params.length - b.params.length);
    const sig = withParams[0];

    // Build CM snippet template: name(${1:param1}, ${2:param2})
    const paramSnippets = sig.params.map((p, i) => `\${${i + 1}:${p}}`);
    const snippetTemplate = `${funcName}(${paramSnippets.join(", ")})`;

    snippet(snippetTemplate)(_view, completion, from, to);
  };
}
