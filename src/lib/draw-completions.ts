import type { CompletionContext, CompletionResult } from "@codemirror/autocomplete";
import { findEnclosingCall } from "./param-tracker";
import { DRAW_CONTEXT_FUNCTIONS, STYLE_OPTIONS } from "./draw-completions.generated";

/**
 * Completion source that provides context-aware suggestions when the cursor
 * is inside ax_draw2d(...), ax_draw3d(...), ax_plot2d(...), or ax_polar(...).
 * Suggests draw objects, style options, and layout options.
 */
export function drawContextCompletionSource(
  context: CompletionContext
): CompletionResult | null {
  const text = context.state.doc.toString();
  const pos = context.pos;

  const call = findEnclosingCall(text, pos);
  if (!call) return null;

  const ctx = DRAW_CONTEXT_FUNCTIONS[call.funcName];
  if (!ctx) return null;

  // Match word prefix at cursor
  const word = context.matchBefore(/[a-zA-Z_][a-zA-Z_0-9]*/);
  if (!word || word.text.length < 1) return null;

  const allOptions = [...ctx.objects, ...STYLE_OPTIONS, ...ctx.layout];

  return {
    from: word.from,
    options: allOptions,
    filter: true,
  };
}
