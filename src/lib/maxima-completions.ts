import type { CompletionContext, CompletionResult } from "@codemirror/autocomplete";
import { snippet } from "@codemirror/autocomplete";
import { completeFunction, getFunction } from "./catalog-client";
import { parseSignature } from "./signature-parser";

export function maximaCompletionSource(autocompleteMode: string) {
  return async (context: CompletionContext): Promise<CompletionResult | null> => {
    const word = context.matchBefore(/[a-zA-Z_][a-zA-Z_0-9]*/);
    if (!word || word.text.length < 2) return null;

    const results = await completeFunction(word.text);
    if (results.length === 0) return null;

    return {
      from: word.from,
      options: results.slice(0, 8).map((r) => ({
        label: r.name,
        detail: r.signature,
        info: r.description || undefined,
        type: "function" as const,
        apply: autocompleteMode === "snippet"
          ? createSnippetApply(r.name)
          : `${r.name}(`,
      })),
    };
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
